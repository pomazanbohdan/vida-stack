#!/usr/bin/env python3
"""Canonical external-subagent dispatch wrapper for VIDA."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
LOG_DIR = ROOT_DIR / ".vida" / "logs"
RUN_LOG_PATH = LOG_DIR / "subagent-runs.jsonl"
ROUTE_RECEIPT_DIR = LOG_DIR / "route-receipts"
ISSUE_CONTRACT_DIR = LOG_DIR / "issue-contracts"


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


vida_config = load_module("vida_config_runtime_dispatch", SCRIPT_DIR / "vida-config.py")
subagent_system = load_module("subagent_system_runtime_dispatch", SCRIPT_DIR / "subagent-system.py")
worker_packet_gate = load_module("worker_packet_gate_runtime_dispatch", SCRIPT_DIR / "worker-packet-gate.py")

DEFAULT_PROJECT_PREFLIGHT_DOC = "docs/process/project-operations.md"
WORKER_MACHINE_READABLE_TEMPLATE = {
    "status": "done",
    "question_answered": "yes",
    "answer": "direct bounded answer",
    "evidence_refs": ["path/to/file:12", "command -> key line"],
    "changed_files": ["path/a", "path/b"],
    "verification_commands": ["exact command"],
    "verification_results": ["command -> pass|fail"],
    "merge_ready": "yes",
    "blockers": [],
    "notes": "short note",
    "recommended_next_action": "concise next step",
    "impact_analysis": {
        "affected_scope": ["bounded files/modules"],
        "contract_impact": ["impact or none"],
        "follow_up_actions": ["follow-up or none"],
        "residual_risks": ["risk or none"],
    },
}
COACH_MACHINE_READABLE_TEMPLATE = {
    **WORKER_MACHINE_READABLE_TEMPLATE,
    "merge_ready": "yes",
    "notes": "coach notes or verification limitations",
    "recommended_next_action": "approve_for_independent_verification_or_return_for_rework",
    "coach_decision": "approved|return_for_rework",
    "rework_required": "yes|no",
    "coach_feedback": "concise coach feedback for the writer",
}
ISSUE_CONTRACT_TEMPLATE = {
    "classification": "defect_equivalent|defect_needs_contract_update|feature_delta|as_designed|not_a_bug|insufficient_evidence",
    "equivalence_assessment": "equivalent_fix|spec_delta_required|as_designed|not_a_bug|insufficient_evidence",
    "reported_behavior": "what is happening now",
    "expected_behavior": "what should happen",
    "scope_in": ["bounded behavioral scope that may change"],
    "scope_out": ["related areas that must not change"],
    "acceptance_checks": ["direct acceptance checks for the writer"],
    "spec_sync_targets": ["spec/docs targets to update if this path proceeds"],
    "wvp_required": "yes|no",
    "wvp_status": "validated|not_required|conflicting|unknown",
}
ANALYSIS_MACHINE_READABLE_TEMPLATE = {
    **WORKER_MACHINE_READABLE_TEMPLATE,
    "recommended_next_action": "proceed_to_writer_or_route_to_spec_delta",
    "issue_contract": ISSUE_CONTRACT_TEMPLATE,
}


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def ensure_dirs() -> None:
    LOG_DIR.mkdir(parents=True, exist_ok=True)


def append_jsonl(path: Path, payload: dict[str, Any]) -> None:
    ensure_dirs()
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(payload, sort_keys=True) + "\n")


def read_prompt(prompt_file: Path) -> str:
    if not prompt_file.exists():
        raise FileNotFoundError(f"Prompt file not found: {prompt_file}")
    return prompt_file.read_text(encoding="utf-8")


def project_preflight_doc() -> str:
    try:
        cfg = vida_config.load_validated_config()
    except Exception:
        return DEFAULT_PROJECT_PREFLIGHT_DOC
    bootstrap_cfg = vida_config.dotted_get(cfg, "project_bootstrap", {}) or {}
    if not isinstance(bootstrap_cfg, dict):
        return DEFAULT_PROJECT_PREFLIGHT_DOC
    value = str(bootstrap_cfg.get("project_operations_doc", DEFAULT_PROJECT_PREFLIGHT_DOC)).strip()
    return value or DEFAULT_PROJECT_PREFLIGHT_DOC


def worker_machine_readable_contract() -> str:
    return json.dumps(WORKER_MACHINE_READABLE_TEMPLATE, indent=2)


def coach_machine_readable_contract() -> str:
    return json.dumps(COACH_MACHINE_READABLE_TEMPLATE, indent=2)


def analysis_machine_readable_contract() -> str:
    return json.dumps(ANALYSIS_MACHINE_READABLE_TEMPLATE, indent=2)


def worker_packet_lines(*, blocking_question: str) -> list[str]:
    return [
        "Runtime Role Packet:",
        "- worker_lane_confirmed: true",
        "- worker_role: subagent",
        "- orchestrator_entry_fallback: _vida/docs/ORCHESTRATOR-ENTRY.MD",
        f"- worker_entry: {worker_packet_gate.SUBAGENT_ENTRY_DOC}",
        f"- worker_thinking: {worker_packet_gate.SUBAGENT_THINKING_DOC}",
        "- impact_tail_policy: required_for_non_stc",
        "- impact_analysis_scope: bounded_to_assigned_scope",
        "Worker Entry Contract:",
        "- You are a bounded worker, not the orchestrator.",
        f"- Follow {worker_packet_gate.SUBAGENT_ENTRY_DOC} as the worker-level entry contract.",
        f"- Follow {worker_packet_gate.SUBAGENT_THINKING_DOC} as the worker thinking subset.",
        "- Do not bootstrap repository-wide orchestration policy.",
        "- Stay inside the provided scope and return evidence in the requested format.",
        "- Prefer concrete findings over workflow narration.",
        f"Blocking Question: {blocking_question}",
    ]


def writer_prompt_text(
    original_prompt: str,
    writer_task_class: str,
    issue_contract: dict[str, Any],
) -> str:
    blocking_question = (
        "What is the minimal implementation and verification change-set that satisfies the normalized issue contract?"
    )
    normalized_contract = normalized_issue_contract(
        "writer-prompt",
        writer_task_class,
        {},
        issue_contract,
        source_subagents=[],
        source_output_files=[],
        evidence_refs=normalized_string_list(issue_contract.get("evidence_refs")),
    )
    lines = [
        *worker_packet_lines(blocking_question=blocking_question),
        f"Task: Implement the requested change for {writer_task_class}.",
        "Mode: WRITE within the approved bounded scope.",
        "Scope: original prompt/spec plus the normalized issue contract below.",
        "Constraints:",
        f"- Follow project preflight from {project_preflight_doc()} before analyze/test/build commands.",
        "- Use STC by default; use PR-CoT only if a bounded implementation trade-off appears inside scope.",
        "- Read target files before editing and keep the change-set inside the approved scope.",
        "- Treat `issue_contract.scope_in` as the allowed behavior surface and `issue_contract.scope_out` as forbidden change area.",
        "- Do not widen task ownership or rewrite orchestration decisions.",
        "- Machine-readable summaries must always include impact_analysis; for STC keep it minimal, and for PR-CoT or MAR populate bounded downstream effects.",
        "Verification:",
        "- Run only the bounded verification commands needed to prove the changed scope.",
        "Deliverable:",
        "- Return the machine-readable summary below.",
        "```json",
        worker_machine_readable_contract(),
        "```",
        "",
        "Original prompt/spec:",
        "<<<PROMPT",
        original_prompt.strip(),
        "PROMPT",
        "",
        "Normalized issue contract:",
        "```json",
        json.dumps(normalized_contract, indent=2, sort_keys=True),
        "```",
    ]
    return "\n".join(lines).strip() + "\n"


def write_writer_issue_contract_prompt(
    output_dir: Path,
    *,
    original_prompt: str,
    writer_task_class: str,
    issue_contract: dict[str, Any],
) -> Path:
    output_dir.mkdir(parents=True, exist_ok=True)
    prompt_path = output_dir / "writer.issue-contract.prompt.txt"
    prompt_path.write_text(
        writer_prompt_text(
            original_prompt=original_prompt,
            writer_task_class=writer_task_class,
            issue_contract=issue_contract,
        ),
        encoding="utf-8",
    )
    return prompt_path


def packet_merge_contract(
    prompt_text: str,
    output_text: str,
    min_output_bytes: int,
) -> tuple[bool, bool, list[str], bool]:
    machine_required = worker_packet_gate.machine_readable_contract_required(prompt_text)
    if not machine_required:
        merge_ready = output_is_merge_ready(output_text, min_output_bytes)
        return merge_ready, output_has_useful_progress(output_text, "", min_output_bytes), [], False
    errors = list(worker_packet_gate.validate_output_text(prompt_text, output_text))
    output_size_ok = len(output_text.strip().encode("utf-8")) >= max(1, min_output_bytes)
    if output_size_ok is False:
        errors.append(f"worker output below min_output_bytes ({len(output_text.strip().encode('utf-8'))}<{max(1, min_output_bytes)})")
    payload = worker_packet_gate.extract_json_payload(output_text) or {}
    declared_merge_ready = str(payload.get("merge_ready", "")).strip().casefold() == "yes"
    return output_size_ok and not errors and declared_merge_ready, not errors, errors, True


def normalize_arg_list(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, str):
        return [item.strip() for item in value.split(",") if item.strip()]
    if isinstance(value, list):
        out: list[str] = []
        for item in value:
            text = str(item).strip()
            if text:
                out.append(text)
        return out
    return []


def selected_model_for_subagent(route: dict[str, Any], subagent_name: str, subagent_cfg: dict[str, Any]) -> str | None:
    if route.get("selected_subagent") == subagent_name:
        return route.get("selected_model")
    for item in route.get("fallback_subagents", []):
        if item.get("subagent") == subagent_name:
            return item.get("selected_model")
    default_model = subagent_cfg.get("default_model")
    return default_model if isinstance(default_model, str) and default_model else None


def route_subagent_item(route: dict[str, Any], subagent_name: str) -> dict[str, Any]:
    if route.get("selected_subagent") == subagent_name:
        return route
    for item in route.get("fallback_subagents", []):
        if item.get("subagent") == subagent_name:
            return item
    return {}


def subagent_command(
    subagent_name: str,
    prompt: str,
    output_path: Path,
    workdir: Path,
    model: str | None,
    subagent_cfg: dict[str, Any],
    route: dict[str, Any] | None = None,
) -> tuple[list[str], bool]:
    dispatch_cfg = subagent_cfg.get("dispatch", {})
    if not subagent_supports_dispatch(subagent_cfg):
        raise ValueError(f"Cli subagent does not expose dispatch config: {subagent_name}")

    command = str(dispatch_cfg.get("command", "")).strip()
    pre_static_args = normalize_arg_list(dispatch_cfg.get("pre_static_args"))
    subcommand = str(dispatch_cfg.get("subcommand", "")).strip()
    static_args = normalize_arg_list(dispatch_cfg.get("static_args"))
    write_static_args = normalize_arg_list(dispatch_cfg.get("write_static_args"))
    route_write_scope = policy_value((route or {}).get("write_scope"), "none")
    subagent_write_scope = policy_value(subagent_cfg.get("write_scope"), "none")
    if route_write_scope != "none" and subagent_write_scope != "none" and write_static_args:
        static_args = write_static_args
    cmd = [command, *pre_static_args]

    web_search_required = policy_value((route or {}).get("web_search_required"), "no") == "yes"
    web_search_mode = str(dispatch_cfg.get("web_search_mode", "none")).strip() or "none"
    if web_search_required and web_search_mode == "flag":
        web_search_flag = str(dispatch_cfg.get("web_search_flag", "")).strip()
        if not web_search_flag:
            raise ValueError(f"Cli subagent dispatch web_search_flag missing for flag mode: {subagent_name}")
        cmd.append(web_search_flag)

    if subcommand:
        cmd.append(subcommand)
    cmd.extend(static_args)

    workdir_flag = str(dispatch_cfg.get("workdir_flag", "")).strip()
    if workdir_flag:
        cmd.extend([workdir_flag, str(workdir)])

    model_flag = str(dispatch_cfg.get("model_flag", "")).strip()
    if model and model_flag:
        cmd.extend([model_flag, model])

    output_mode = str(dispatch_cfg.get("output_mode", "stdout")).strip() or "stdout"
    if output_mode == "file":
        output_flag = str(dispatch_cfg.get("output_flag", "")).strip()
        if not output_flag:
            raise ValueError(f"Cli subagent dispatch output_flag missing for file mode: {subagent_name}")
        cmd.extend([output_flag, str(output_path)])
        use_stdout_output = False
    elif output_mode == "stdout":
        use_stdout_output = True
    else:
        raise ValueError(f"Unsupported cli subagent dispatch output_mode={output_mode}: {subagent_name}")

    prompt_mode = str(dispatch_cfg.get("prompt_mode", "positional")).strip() or "positional"
    if prompt_mode == "flag":
        prompt_flag = str(dispatch_cfg.get("prompt_flag", "")).strip()
        if not prompt_flag:
            raise ValueError(f"Cli subagent dispatch prompt_flag missing for flag mode: {subagent_name}")
        cmd.extend([prompt_flag, prompt])
    elif prompt_mode == "positional":
        cmd.append(prompt)
    else:
        raise ValueError(f"Unsupported cli subagent dispatch prompt_mode={prompt_mode}: {subagent_name}")
    return cmd, use_stdout_output


def subagent_env(subagent_cfg: dict[str, Any]) -> dict[str, str]:
    dispatch_cfg = subagent_cfg.get("dispatch", {})
    raw_env = dispatch_cfg.get("env", {})
    if not isinstance(raw_env, dict):
        return {}
    env: dict[str, str] = {}
    for key, value in raw_env.items():
        name = str(key).strip()
        if not name:
            continue
        env[name] = str(value)
    return env


def subagent_supports_dispatch(subagent_cfg: dict[str, Any]) -> bool:
    dispatch_cfg = subagent_cfg.get("dispatch", {})
    if not isinstance(dispatch_cfg, dict):
        return False
    command = dispatch_cfg.get("command")
    return isinstance(command, str) and bool(command.strip())


def output_size(path: Path) -> int:
    if not path.exists():
        return 0
    try:
        return path.stat().st_size
    except OSError:
        return 0


def policy_int(value: Any, default: int) -> int:
    if value is None:
        return default
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def policy_value(value: Any, default: str) -> str:
    if value is None:
        return default
    if isinstance(value, str):
        trimmed = value.strip()
        return trimmed if trimmed else default
    return str(value)


def route_runtime_limit(route: dict[str, Any], subagent_cfg: dict[str, Any]) -> int:
    route_limit = policy_int(route.get("max_runtime_seconds"), 0)
    subagent_limit = policy_int(subagent_cfg.get("max_runtime_seconds"), 0)
    return route_limit or subagent_limit or 180


def dispatch_runtime_limit(
    base_limit: int,
    dispatch_mode: str,
    subagent_route: dict[str, Any],
    subagent_cfg: dict[str, Any],
) -> int:
    limit = max(60, base_limit)
    orchestration_tier = policy_value(subagent_route.get("orchestration_tier"), policy_value(subagent_cfg.get("orchestration_tier"), "standard"))
    quality_tier = policy_value(subagent_cfg.get("quality_tier"), "medium")
    if dispatch_mode == "fallback":
        if orchestration_tier == "bridge":
            limit = max(limit, 240)
        if quality_tier == "high":
            limit = max(limit, 220)
    if dispatch_mode == "arbitration":
        limit = min(limit, 180)
    return limit


def route_min_output_bytes(route: dict[str, Any], subagent_cfg: dict[str, Any]) -> int:
    route_min = policy_int(route.get("min_output_bytes"), 0)
    subagent_min = policy_int(subagent_cfg.get("min_output_bytes"), 0)
    return route_min or subagent_min or 220


def dispatch_timeout_seconds(subagent_cfg: dict[str, Any], key: str, default: int) -> int:
    dispatch_cfg = subagent_cfg.get("dispatch", {})
    if not isinstance(dispatch_cfg, dict):
        return default
    return max(5, policy_int(dispatch_cfg.get(key), default))


def route_risk_class(route: dict[str, Any]) -> str:
    value = str(route.get("risk_class", "R0")).strip().upper()
    return value if value in {"R0", "R1", "R2", "R3", "R4"} else "R0"


def route_dispatch_policy(route: dict[str, Any]) -> dict[str, Any]:
    policy = route.get("dispatch_policy", {})
    return policy if isinstance(policy, dict) else {}


def route_budget_policy(route: dict[str, Any]) -> dict[str, Any]:
    budget = route.get("route_budget", {})
    return budget if isinstance(budget, dict) else {}


def route_budget_blockers(route: dict[str, Any]) -> list[str]:
    budget = route_budget_policy(route)
    blockers: list[str] = []
    max_budget_units = int(budget.get("max_budget_units", 0) or 0)
    estimated_route_cost_units = int(budget.get("estimated_route_cost_units", 0) or 0)
    max_cli_subagent_calls = int(budget.get("max_cli_subagent_calls", 0) or 0)
    estimated_cli_calls = (
        int(budget.get("estimated_primary_calls", 0) or 0)
        + int(budget.get("estimated_coach_calls", 0) or 0)
        + int(budget.get("estimated_verification_calls", 0) or 0)
    )
    max_coach_passes = int(budget.get("max_coach_passes", 0) or 0)
    estimated_coach_calls = int(budget.get("estimated_coach_calls", 0) or 0)
    max_verification_passes = int(budget.get("max_verification_passes", 0) or 0)
    estimated_verification_calls = int(budget.get("estimated_verification_calls", 0) or 0)
    max_fallback_hops = int(budget.get("max_fallback_hops", 0) or 0)
    estimated_fallback_hops = int(budget.get("estimated_fallback_hops", 0) or 0)

    if max_budget_units > 0 and estimated_route_cost_units > max_budget_units:
        blockers.append("route_budget_cap_exceeded")
    if max_cli_subagent_calls > 0 and estimated_cli_calls > max_cli_subagent_calls:
        blockers.append("cli_subagent_call_cap_exceeded")
    if estimated_coach_calls > max_coach_passes:
        blockers.append("coach_pass_cap_exceeded")
    if estimated_verification_calls > max_verification_passes:
        blockers.append("verification_pass_cap_exceeded")
    if estimated_fallback_hops > max_fallback_hops:
        blockers.append("fallback_hop_cap_exceeded")
    return blockers


def route_policy_payload(
    task_id: str,
    route: dict[str, Any],
    subagent_name: str,
    subagent_cfg: dict[str, Any],
    dispatch_mode: str,
) -> dict[str, Any]:
    dispatch_policy = route_dispatch_policy(route)
    route_budget = route_budget_policy(route)
    selected_budget_units = int(subagent_cfg.get("budget_cost_units", 0) or 0)
    selected_cost_class = subagent_system.cost_class_for_units(selected_budget_units)
    internal_escalation_used = subagent_name == "internal_subagents" and dispatch_mode in {"fallback", "arbitration"}
    internal_authorized = (
        dispatch_policy.get("internal_route_authorized") == "yes"
        or route.get("internal_route_authorized") == "yes"
    )
    bridge_fallback_used = dispatch_mode == "fallback" and subagent_name == route.get("bridge_fallback_subagent")
    cheap_lane_attempted = (
        dispatch_mode in {"fanout", "single", "fallback"}
        and policy_value(subagent_cfg.get("billing_tier"), "unknown") in {"free", "low"}
    )
    max_budget_units = int(route_budget.get("max_budget_units", 0) or 0)
    budget_violation = max_budget_units > 0 and selected_budget_units > max_budget_units
    cost_escalation_trigger = ""
    if bridge_fallback_used:
        cost_escalation_trigger = "bridge_fallback"
    if internal_escalation_used:
        cost_escalation_trigger = policy_value(route.get("internal_escalation_trigger"), "internal_escalation")
    internal_escalation_receipt: dict[str, Any] = {}
    internal_escalation_receipt_error = ""
    if internal_escalation_used:
        internal_receipt_ok, receipt, internal_escalation_receipt_error = validate_internal_escalation_receipt(task_id, route)
        if internal_receipt_ok:
            internal_escalation_receipt = {
                **receipt,
                "allowed_reasons": dispatch_policy.get("allowed_internal_reasons", []),
                "required_dispatch_path": dispatch_policy.get("required_dispatch_path", []),
            }
            internal_authorized = True
        else:
            internal_authorized = False
    policy_bypass = False
    if (
        subagent_name == "internal_subagents"
        and (
            (dispatch_policy.get("direct_internal_bypass_forbidden") == "yes" and not internal_authorized)
            or (internal_escalation_used and not internal_authorized)
        )
    ):
        policy_bypass = True
    return {
        "selected_budget_units": selected_budget_units,
        "selected_cost_class": selected_cost_class,
        "route_budget_policy": policy_value(route_budget.get("budget_policy"), "balanced"),
        "route_budget_max_units": max_budget_units,
        "route_budget_max_cost_class": policy_value(route_budget.get("max_budget_cost_class"), "free"),
        "route_estimated_cost_class": policy_value(route_budget.get("estimated_route_cost_class"), "free"),
        "cheap_lane_attempted": cheap_lane_attempted,
        "bridge_fallback_used": bridge_fallback_used,
        "internal_escalation_used": internal_escalation_used,
        "internal_route_authorized": internal_authorized,
        "policy_bypass": policy_bypass,
        "budget_violation": budget_violation,
        "cost_escalation_trigger": cost_escalation_trigger,
        "internal_escalation_receipt_error": internal_escalation_receipt_error,
        "internal_escalation_receipt": internal_escalation_receipt,
    }


def route_receipt_payload(route: dict[str, Any]) -> dict[str, Any]:
    analysis_plan = route.get("analysis_plan", {})
    coach_plan = route.get("coach_plan", {})
    verification_plan = route.get("verification_plan", {})
    return {
        "task_class": route.get("task_class"),
        "dispatch_required": route.get("dispatch_required"),
        "external_first_required": route.get("external_first_required"),
        "web_search_required": route.get("web_search_required", "no"),
        "analysis_required": route.get("analysis_required"),
        "analysis_route_task_class": route.get("analysis_route_task_class"),
        "analysis_receipt_required": route.get("analysis_receipt_required"),
        "analysis_zero_budget_required": route.get("analysis_zero_budget_required"),
        "analysis_default_in_boot": route.get("analysis_default_in_boot"),
        "coach_required": route.get("coach_required", "no"),
        "coach_route_task_class": route.get("coach_route_task_class", ""),
        "analysis_plan": {
            "required": analysis_plan.get("required", "no"),
            "route_task_class": analysis_plan.get("route_task_class"),
            "selected_subagent": analysis_plan.get("selected_subagent"),
            "fanout_subagents": analysis_plan.get("fanout_subagents", []),
            "fanout_min_results": int(analysis_plan.get("fanout_min_results", 0) or 0),
            "merge_policy": analysis_plan.get("merge_policy"),
            "external_first_required": analysis_plan.get("external_first_required"),
            "zero_budget_required": analysis_plan.get("zero_budget_required"),
            "receipt_required": analysis_plan.get("receipt_required"),
            "default_in_boot": analysis_plan.get("default_in_boot"),
            "reason": analysis_plan.get("reason", ""),
        },
        "coach_plan": {
            "required": coach_plan.get("required", "no"),
            "route_task_class": coach_plan.get("route_task_class"),
            "selected_subagent": coach_plan.get("selected_subagent"),
            "selected_subagents": coach_plan.get("selected_subagents", []),
            "independent": bool(coach_plan.get("independent", False)),
            "min_results": int(coach_plan.get("min_results", 0) or 0),
            "merge_policy": coach_plan.get("merge_policy", "single_subagent"),
            "max_passes": int(coach_plan.get("max_passes", 0) or 0),
            "reason": coach_plan.get("reason", ""),
        },
        "fanout_subagents": route.get("fanout_subagents", []),
        "fanout_min_results": int(route.get("fanout_min_results", 0) or 0),
        "merge_policy": route.get("merge_policy"),
        "bridge_fallback_subagent": route.get("bridge_fallback_subagent"),
        "internal_escalation_trigger": route.get("internal_escalation_trigger"),
        "independent_verification_required": route.get("independent_verification_required"),
        "verification_route_task_class": route.get("verification_route_task_class"),
        "verification_plan": {
            "required": verification_plan.get("required", "no"),
            "route_task_class": verification_plan.get("route_task_class"),
            "selected_subagent": verification_plan.get("selected_subagent"),
            "independent": bool(verification_plan.get("independent", False)),
            "reason": verification_plan.get("reason", ""),
        },
        "dispatch_policy": route.get("dispatch_policy", {}),
        "route_graph": route.get("route_graph", {}),
        "route_budget": route.get("route_budget", {}),
    }


def route_receipt_path(task_id: str, task_class: str) -> Path:
    safe_task_id = re.sub(r"[^A-Za-z0-9._-]+", "-", task_id.strip() or "task")
    safe_task_class = re.sub(r"[^A-Za-z0-9._-]+", "-", task_class.strip() or "task_class")
    return ROUTE_RECEIPT_DIR / f"{safe_task_id}.{safe_task_class}.route.json"


def issue_contract_path(task_id: str) -> Path:
    safe_task_id = re.sub(r"[^A-Za-z0-9._-]+", "-", task_id.strip() or "task")
    return ISSUE_CONTRACT_DIR / f"{safe_task_id}.json"


def route_receipt_hash(route: dict[str, Any]) -> str:
    return digest_text(json.dumps(route_receipt_payload(route), sort_keys=True))


def write_route_receipt(task_id: str, task_class: str, route: dict[str, Any]) -> Path:
    path = route_receipt_path(task_id, task_class)
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "status": "route_selected",
        "route_receipt": route_receipt_payload(route),
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def analysis_receipt_path(task_id: str, task_class: str) -> Path:
    safe_task_id = re.sub(r"[^A-Za-z0-9._-]+", "-", task_id.strip() or "task")
    safe_task_class = re.sub(r"[^A-Za-z0-9._-]+", "-", task_class.strip() or "task_class")
    return ROUTE_RECEIPT_DIR / f"{safe_task_id}.{safe_task_class}.analysis.json"


def analysis_blocker_path(task_id: str, task_class: str) -> Path:
    safe_task_id = re.sub(r"[^A-Za-z0-9._-]+", "-", task_id.strip() or "task")
    safe_task_class = re.sub(r"[^A-Za-z0-9._-]+", "-", task_class.strip() or "task_class")
    return ROUTE_RECEIPT_DIR / f"{safe_task_id}.{safe_task_class}.analysis-blocker.json"


def coach_receipt_path(task_id: str, task_class: str) -> Path:
    safe_task_id = re.sub(r"[^A-Za-z0-9._-]+", "-", task_id.strip() or "task")
    safe_task_class = re.sub(r"[^A-Za-z0-9._-]+", "-", task_class.strip() or "task_class")
    return ROUTE_RECEIPT_DIR / f"{safe_task_id}.{safe_task_class}.coach.json"


def coach_blocker_path(task_id: str, task_class: str) -> Path:
    safe_task_id = re.sub(r"[^A-Za-z0-9._-]+", "-", task_id.strip() or "task")
    safe_task_class = re.sub(r"[^A-Za-z0-9._-]+", "-", task_class.strip() or "task_class")
    return ROUTE_RECEIPT_DIR / f"{safe_task_id}.{safe_task_class}.coach-blocker.json"


def rework_handoff_path(task_id: str, task_class: str) -> Path:
    safe_task_id = re.sub(r"[^A-Za-z0-9._-]+", "-", task_id.strip() or "task")
    safe_task_class = re.sub(r"[^A-Za-z0-9._-]+", "-", task_class.strip() or "task_class")
    return ROUTE_RECEIPT_DIR / f"{safe_task_id}.{safe_task_class}.rework-handoff.json"


def internal_escalation_receipt_path(task_id: str, task_class: str) -> Path:
    safe_task_id = re.sub(r"[^A-Za-z0-9._-]+", "-", task_id.strip() or "task")
    safe_task_class = re.sub(r"[^A-Za-z0-9._-]+", "-", task_class.strip() or "task_class")
    return ROUTE_RECEIPT_DIR / f"{safe_task_id}.{safe_task_class}.internal-escalation.json"


def load_analysis_receipt(task_id: str, task_class: str) -> dict[str, Any]:
    path = analysis_receipt_path(task_id, task_class)
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def load_analysis_blocker(task_id: str, task_class: str) -> dict[str, Any]:
    path = analysis_blocker_path(task_id, task_class)
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def load_coach_receipt(task_id: str, task_class: str) -> dict[str, Any]:
    path = coach_receipt_path(task_id, task_class)
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def load_coach_blocker(task_id: str, task_class: str) -> dict[str, Any]:
    path = coach_blocker_path(task_id, task_class)
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def load_rework_handoff(task_id: str, task_class: str) -> dict[str, Any]:
    path = rework_handoff_path(task_id, task_class)
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def load_issue_contract(task_id: str) -> dict[str, Any]:
    path = issue_contract_path(task_id)
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def load_internal_escalation_receipt(task_id: str, task_class: str) -> dict[str, Any]:
    path = internal_escalation_receipt_path(task_id, task_class)
    if not path.exists():
        return {}
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return payload if isinstance(payload, dict) else {}


def clear_analysis_blocker(task_id: str, task_class: str) -> None:
    path = analysis_blocker_path(task_id, task_class)
    try:
        if path.exists():
            path.unlink()
    except OSError:
        return


def clear_analysis_receipt(task_id: str, task_class: str) -> None:
    path = analysis_receipt_path(task_id, task_class)
    try:
        if path.exists():
            path.unlink()
    except OSError:
        return


def clear_coach_receipt(task_id: str, task_class: str) -> None:
    path = coach_receipt_path(task_id, task_class)
    try:
        if path.exists():
            path.unlink()
    except OSError:
        return


def clear_coach_blocker(task_id: str, task_class: str) -> None:
    path = coach_blocker_path(task_id, task_class)
    try:
        if path.exists():
            path.unlink()
    except OSError:
        return


def clear_rework_handoff(task_id: str, task_class: str) -> None:
    path = rework_handoff_path(task_id, task_class)
    try:
        if path.exists():
            path.unlink()
    except OSError:
        return


def issue_contract_status(classification: str, equivalence_assessment: str) -> str:
    normalized_equivalence = policy_value(equivalence_assessment, "").casefold()
    normalized_classification = policy_value(classification, "").casefold()
    if normalized_equivalence == "equivalent_fix":
        return "writer_ready"
    if normalized_equivalence == "spec_delta_required":
        return "spec_delta_required"
    if normalized_equivalence in {"as_designed", "not_a_bug"}:
        return "issue_closed_no_fix"
    if normalized_classification == "defect_equivalent":
        return "writer_ready"
    if normalized_classification in {"defect_needs_contract_update", "feature_delta"}:
        return "spec_delta_required"
    if normalized_classification in {"as_designed", "not_a_bug"}:
        return "issue_closed_no_fix"
    return "insufficient_evidence"


def issue_contract_resolution_path(status: str) -> str:
    if status == "writer_ready":
        return "implementation"
    if status == "spec_delta_required":
        return "spec_reconciliation"
    if status == "issue_closed_no_fix":
        return "close_without_writer"
    return "blocked"


def normalized_issue_contract(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
    raw_issue_contract: dict[str, Any],
    *,
    source_subagents: list[str],
    source_output_files: list[str],
    evidence_refs: list[str],
    conflict_statuses: list[str] | None = None,
) -> dict[str, Any]:
    classification = policy_value(raw_issue_contract.get("classification"), "insufficient_evidence")
    equivalence_assessment = policy_value(raw_issue_contract.get("equivalence_assessment"), "")
    status = issue_contract_status(classification, equivalence_assessment)
    return {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "status": status,
        "classification": classification,
        "equivalence_assessment": equivalence_assessment,
        "reported_behavior": policy_value(raw_issue_contract.get("reported_behavior"), ""),
        "expected_behavior": policy_value(raw_issue_contract.get("expected_behavior"), ""),
        "scope_in": normalized_string_list(raw_issue_contract.get("scope_in")),
        "scope_out": normalized_string_list(raw_issue_contract.get("scope_out")),
        "acceptance_checks": normalized_string_list(raw_issue_contract.get("acceptance_checks")),
        "spec_sync_targets": normalized_string_list(raw_issue_contract.get("spec_sync_targets")),
        "wvp_required": policy_value(raw_issue_contract.get("wvp_required"), "no"),
        "wvp_status": policy_value(raw_issue_contract.get("wvp_status"), "unknown"),
        "resolution_path": issue_contract_resolution_path(status),
        "route_receipt_hash": route_receipt_hash(route),
        "route_receipt": route_receipt_payload(route),
        "source_subagents": deduped_strings(source_subagents),
        "source_output_files": deduped_strings(source_output_files),
        "evidence_refs": deduped_strings(evidence_refs),
        "conflict_statuses": deduped_strings(conflict_statuses or []),
    }


def write_issue_contract(task_id: str, payload: dict[str, Any]) -> Path:
    path = issue_contract_path(task_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def validate_issue_contract(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
) -> tuple[bool, dict[str, Any], str]:
    if task_class != "implementation":
        return True, {}, ""

    payload = load_issue_contract(task_id)
    if not payload:
        return False, {}, "missing_issue_contract"

    status = policy_value(payload.get("status"), "")
    if not status:
        return False, payload, "missing_issue_contract_status"
    if payload.get("route_receipt_hash") != route_receipt_hash(route):
        return False, payload, "stale_issue_contract"
    if status == "writer_ready":
        return True, payload, ""
    if status in {"spec_delta_required", "issue_closed_no_fix", "insufficient_evidence"}:
        return False, payload, status
    return False, payload, "invalid_issue_contract_status"


def validate_internal_escalation_receipt(
    task_id: str,
    route: dict[str, Any],
) -> tuple[bool, dict[str, Any], str]:
    task_class = policy_value(route.get("task_class"), "")
    if not task_class:
        return False, {}, "missing_internal_escalation_task_class"

    receipt = load_internal_escalation_receipt(task_id, task_class)
    if not receipt:
        return False, {}, "missing_internal_escalation_receipt"

    reason = policy_value(receipt.get("reason"), policy_value(receipt.get("trigger"), ""))
    if not reason:
        return False, receipt, "missing_internal_escalation_reason"

    allowed_reasons = {
        policy_value(route.get("internal_escalation_trigger"), ""),
        *[policy_value(item, "") for item in route_dispatch_policy(route).get("allowed_internal_reasons", [])],
    }
    allowed_reasons.discard("")
    if allowed_reasons and reason not in allowed_reasons:
        return False, receipt, "invalid_internal_escalation_reason"

    if not policy_value(receipt.get("scope"), ""):
        return False, receipt, "missing_internal_escalation_scope"
    if not policy_value(receipt.get("notes"), ""):
        return False, receipt, "missing_internal_escalation_notes"
    if receipt.get("route_receipt_hash") != route_receipt_hash(route):
        return False, receipt, "stale_internal_escalation_receipt"

    return True, receipt, ""


def write_analysis_receipt(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
    manifest: dict[str, Any],
) -> Path:
    path = analysis_receipt_path(task_id, task_class)
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "status": "analysis_ready",
        "analysis_task_class": route.get("task_class"),
        "route_receipt_hash": route_receipt_hash(route),
        "route_receipt": route_receipt_payload(route),
        "manifest_path": manifest.get("manifest_path", ""),
        "manifest_summary": {
            "synthesis_ready": bool(manifest.get("synthesis_ready", False)),
            "decision_ready": bool(manifest.get("decision_ready", False)),
            "subagent_exhausted": bool(manifest.get("subagent_exhausted", False)),
            "results_count": int(manifest.get("result_count", 0) or 0),
        },
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    clear_analysis_blocker(task_id, task_class)
    return path


def write_analysis_blocker(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
    *,
    status: str,
    reason: str,
    prepare_manifest: dict[str, Any],
    analysis_manifest: dict[str, Any] | None = None,
) -> Path:
    path = analysis_blocker_path(task_id, task_class)
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "status": status,
        "reason": reason,
        "analysis_task_class": policy_value((route.get("analysis_plan") or {}).get("route_task_class"), ""),
        "route_receipt": route_receipt_payload(route),
        "route_receipt_hash": route_receipt_hash(route),
        "prepare_manifest_status": prepare_manifest.get("status", ""),
        "analysis_manifest_path": prepare_manifest.get("analysis_manifest_path", ""),
        "analysis_return_code": prepare_manifest.get("analysis_return_code"),
        "analysis_manifest_status": (analysis_manifest or {}).get("status", ""),
        "analysis_manifest_phase": (analysis_manifest or {}).get("phase", ""),
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def coach_attempt_count(task_id: str, task_class: str) -> int:
    prior = [load_coach_receipt(task_id, task_class), load_coach_blocker(task_id, task_class)]
    return max(int(payload.get("attempt_count", 0) or 0) for payload in prior if isinstance(payload, dict)) if prior else 0


def write_coach_receipt(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
    manifest: dict[str, Any],
    *,
    coach_decision: dict[str, Any],
    attempt_count: int,
) -> Path:
    coach_decision = with_coach_feedback_provenance(coach_decision)
    path = coach_receipt_path(task_id, task_class)
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "status": "coach_approved",
        "attempt_count": attempt_count,
        "coach_task_class": policy_value((route.get("coach_plan") or {}).get("route_task_class"), ""),
        "route_receipt_hash": route_receipt_hash(route),
        "route_receipt": route_receipt_payload(route),
        "manifest_path": manifest.get("manifest_path", ""),
        "manifest_summary": {
            "synthesis_ready": bool(manifest.get("synthesis_ready", False)),
            "decision_ready": bool(manifest.get("decision_ready", False)),
            "subagent_exhausted": bool(manifest.get("subagent_exhausted", False)),
            "results_count": int(manifest.get("result_count", 0) or 0),
        },
        "coach_decision": coach_decision,
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    clear_coach_blocker(task_id, task_class)
    return path


def write_coach_blocker(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
    *,
    status: str,
    reason: str,
    attempt_count: int,
    coach_manifest: dict[str, Any] | None = None,
    coach_decision: dict[str, Any] | None = None,
    rework_handoff_payload: dict[str, Any] | None = None,
) -> Path:
    normalized_coach_decision = with_coach_feedback_provenance(coach_decision or {})
    path = coach_blocker_path(task_id, task_class)
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "status": status,
        "reason": reason,
        "attempt_count": attempt_count,
        "coach_task_class": policy_value((route.get("coach_plan") or {}).get("route_task_class"), ""),
        "route_receipt": route_receipt_payload(route),
        "route_receipt_hash": route_receipt_hash(route),
        "coach_manifest_path": (coach_manifest or {}).get("manifest_path", ""),
        "coach_manifest_status": (coach_manifest or {}).get("status", ""),
        "coach_manifest_phase": (coach_manifest or {}).get("phase", ""),
        "coach_decision": normalized_coach_decision,
        "rework_handoff_path": (rework_handoff_payload or {}).get("path", ""),
        "rework_handoff_status": (rework_handoff_payload or {}).get("status", ""),
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    clear_coach_receipt(task_id, task_class)
    return path


def normalized_string_list(value: Any) -> list[str]:
    if not isinstance(value, list):
        return []
    items: list[str] = []
    for item in value:
        text = str(item).strip()
        if text:
            items.append(text)
    return items


def normalized_impact_analysis(value: Any) -> dict[str, list[str]]:
    if not isinstance(value, dict):
        return {}
    payload = {
        "affected_scope": normalized_string_list(value.get("affected_scope")),
        "contract_impact": normalized_string_list(value.get("contract_impact")),
        "follow_up_actions": normalized_string_list(value.get("follow_up_actions")),
        "residual_risks": normalized_string_list(value.get("residual_risks")),
    }
    return {key: items for key, items in payload.items() if items}


def deduped_strings(items: list[str]) -> list[str]:
    out: list[str] = []
    for item in items:
        text = str(item).strip()
        if text and text not in out:
            out.append(text)
    return out


def coach_selected_subagents(coach_plan: dict[str, Any]) -> list[str]:
    names = [str(item).strip() for item in coach_plan.get("selected_subagents", []) if str(item).strip()]
    if names:
        return deduped_strings(names)
    selected = str(coach_plan.get("selected_subagent", "")).strip()
    return [selected] if selected else []


def coach_decision_is_valid(decision: dict[str, Any]) -> bool:
    return (
        isinstance(decision, dict)
        and decision.get("parsed_json") is True
        and str(decision.get("coach_decision", "")).strip() in {"approved", "return_for_rework"}
    )


def merge_impact_analyses(decisions: list[dict[str, Any]]) -> dict[str, list[str]]:
    merged = {
        "affected_scope": [],
        "contract_impact": [],
        "follow_up_actions": [],
        "residual_risks": [],
    }
    for decision in decisions:
        impact = normalized_impact_analysis(decision.get("impact_analysis"))
        for key in merged:
            merged[key].extend(normalized_string_list(impact.get(key)))
    merged = {key: deduped_strings(items) for key, items in merged.items()}
    return {key: items for key, items in merged.items() if items}


def coach_feedback_summary(decisions: list[dict[str, Any]]) -> str:
    lines: list[str] = []
    for decision in decisions:
        feedback = (
            str(decision.get("coach_feedback", "")).strip()
            or str(decision.get("reason", "")).strip()
            or str(decision.get("answer", "")).strip()
        )
        if not feedback:
            continue
        subagent = str(decision.get("subagent", "")).strip()
        prefix = f"[{subagent}] " if subagent else ""
        line = f"{prefix}{feedback}"
        if line not in lines:
            lines.append(line)
    return "\n".join(lines)


def coach_feedback_text_candidate(text: str) -> str:
    collapsed = preview_text(text, 420)
    if not collapsed:
        return ""
    if looks_like_planning_chatter(collapsed.casefold()):
        return ""
    return collapsed


def with_coach_feedback_provenance(
    decision: dict[str, Any],
    *,
    primary_source: str = "",
    source_chain: list[str] | None = None,
) -> dict[str, Any]:
    normalized_primary = str(primary_source).strip()
    normalized_chain = deduped_strings(list(source_chain or []))
    existing_primary = str(decision.get("feedback_source", "")).strip()
    existing_chain = deduped_strings(normalized_string_list(decision.get("feedback_sources")))
    primary = normalized_primary or existing_primary
    chain = deduped_strings([*normalized_chain, *existing_chain])
    if not primary:
        if chain:
            primary = chain[0]
        elif decision.get("parsed_json") is True:
            primary = "output_json_payload"
        elif any(str(decision.get(key, "")).strip() for key in ("coach_feedback", "reason", "answer")):
            primary = "output_text"
        else:
            primary = "default_fallback"
    if primary and primary not in chain:
        chain = [primary, *chain]
    enriched = dict(decision)
    enriched["feedback_source"] = primary
    enriched["feedback_sources"] = chain or ([primary] if primary else [])
    return enriched


def validate_rework_handoff(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
) -> tuple[bool, dict[str, Any], str]:
    handoff = load_rework_handoff(task_id, task_class)
    if not handoff:
        return False, {}, "missing_rework_handoff"
    if handoff.get("status") != "writer_rework_ready":
        return False, handoff, "invalid_rework_handoff_status"
    if handoff.get("route_receipt_hash") != route_receipt_hash(route):
        return False, handoff, "stale_rework_handoff"
    if handoff.get("fresh_start_required") is not True:
        return False, handoff, "missing_fresh_start_contract"
    if not str(handoff.get("original_prompt_text", "")).strip():
        return False, handoff, "missing_original_prompt_text"
    if not str(handoff.get("fresh_prompt_text", "")).strip():
        return False, handoff, "missing_fresh_prompt_text"
    coach_delta = handoff.get("coach_delta")
    if not isinstance(coach_delta, dict):
        return False, handoff, "missing_coach_delta"
    feedback_source = str(coach_delta.get("feedback_source", "")).strip()
    feedback_sources = normalized_string_list(coach_delta.get("feedback_sources"))
    if not feedback_source or not feedback_sources or feedback_source not in feedback_sources:
        return False, handoff, "missing_feedback_provenance"
    return True, handoff, ""


def dispatch_policy_violation(
    task_id: str,
    route: dict[str, Any],
    subagent_name: str,
    dispatch_mode: str,
    subagent_cfg: dict[str, Any] | None = None,
) -> str:
    dispatch_required = policy_value(route.get("dispatch_required"), "")
    external_first_required = policy_value(route.get("external_first_required"), "no")
    analysis_plan = route.get("analysis_plan", {})
    dispatch_policy = route_dispatch_policy(route)
    direct_internal_forbidden = dispatch_policy.get("direct_internal_bypass_forbidden") == "yes"
    fanout_subagents = list(route.get("fanout_subagents", []))
    bridge_fallback_subagent = policy_value(route.get("bridge_fallback_subagent"), "")
    internal_escalation_allowed = dispatch_policy.get("internal_escalation_allowed") == "yes"
    analysis_required = analysis_plan.get("required") == "yes"
    analysis_receipt_required = analysis_plan.get("receipt_required") == "yes"
    analysis_route_task_class = policy_value(analysis_plan.get("route_task_class"), "")
    analysis_lane_subagents = {
        str(item).strip()
        for item in [analysis_plan.get("selected_subagent"), *analysis_plan.get("fanout_subagents", [])]
        if str(item).strip()
    }

    if (
        analysis_required
        and analysis_receipt_required
        and analysis_route_task_class
        and route.get("task_class") != analysis_route_task_class
        and subagent_name not in analysis_lane_subagents
        and not load_analysis_receipt(task_id, policy_value(route.get("task_class"), "") or analysis_route_task_class)
    ):
        return "analysis receipt required before writer or bridge dispatch"

    if dispatch_mode == "single" and dispatch_required == "fanout_then_synthesize" and fanout_subagents:
        return "route requires ensemble fanout before synthesis; single dispatch is invalid"
    if (
        dispatch_mode == "single"
        and external_first_required == "yes"
        and fanout_subagents
        and subagent_name not in fanout_subagents
        and subagent_name != bridge_fallback_subagent
    ):
        return "route requires external-first fanout; selected single dispatch is outside the declared path"
    if (
        dispatch_mode == "single"
        and subagent_name == "internal_subagents"
    ):
        internal_receipt_ok, _, internal_receipt_error = validate_internal_escalation_receipt(task_id, route)
        if direct_internal_forbidden:
            return "direct internal bypass is forbidden by route policy"
        if not internal_receipt_ok and dispatch_policy.get("internal_route_authorized") != "yes" and route.get("internal_route_authorized") != "yes":
            return f"internal dispatch requires explicit escalation receipt ({internal_receipt_error})"
    if (
        dispatch_mode in {"fallback", "arbitration"}
        and subagent_name == "internal_subagents"
    ):
        if not internal_escalation_allowed:
            return "internal escalation is not allowed by route policy"
        internal_receipt_ok, _, internal_receipt_error = validate_internal_escalation_receipt(task_id, route)
        if not internal_receipt_ok:
            return f"internal escalation requires explicit escalation receipt ({internal_receipt_error})"
    if subagent_cfg is not None:
        max_budget_units = int(route_budget_policy(route).get("max_budget_units", 0) or 0)
        selected_budget_units = int(subagent_cfg.get("budget_cost_units", 0) or 0)
        if max_budget_units > 0 and selected_budget_units > max_budget_units:
            return f"budget cap exceeded for selected subagent ({selected_budget_units}>{max_budget_units})"
    return ""


def review_state_for(status: str, merge_ready: bool, risk_class: str) -> str:
    if status != "success":
        return "review_failed"
    if not merge_ready:
        return "review_pending"
    return subagent_system.target_review_state_for(risk_class)


def manifest_review_state(summary: dict[str, Any], risk_class: str) -> str:
    if summary.get("subagent_exhausted") and not summary.get("decision_ready"):
        return "review_failed"
    if summary.get("tie_break_recommended") or summary.get("open_conflicts"):
        return "review_pending"
    return subagent_system.target_manifest_review_state_for(risk_class)


def infer_domain_tags(prompt: str, task_class: str) -> list[str]:
    text = f"{task_class}\n{prompt}".casefold()
    tags: list[str] = []
    if any(token in text for token in ["api", "json", "schema", "payload", "endpoint"]):
        tags.append("api_contract")
    if any(token in text for token in ["auth", "session", "token", "bearer", "security"]):
        tags.append("auth_security")
    if any(token in text for token in ["ui", "widget", "layout", "render", "component"]):
        tags.append("frontend_ui")
    if any(token in text for token in ["state", "store", "subagent", "cache", "repository"]):
        tags.append("state_management")
    if any(token in text for token in ["agents.md", "_vida", "protocol", "subagent", "framework"]):
        tags.append("vida_framework")
    if not tags:
        tags.append(task_class)
    return list(dict.fromkeys(tags))


def output_is_merge_ready(text: str, min_output_bytes: int) -> bool:
    stripped = text.strip()
    if len(stripped.encode("utf-8")) < max(1, min_output_bytes):
        return False
    normalized = stripped.casefold()
    if looks_like_planning_chatter(normalized):
        return False
    return structured_evidence_score(normalized) >= 3


def looks_like_planning_chatter(normalized_text: str) -> bool:
    stripped = normalized_text.strip()
    if not stripped:
        return False
    chatter_prefixes = (
        "let me ",
        "now let me ",
        "i will ",
        "i'll ",
        "i am going to ",
        "i will begin by ",
        "i will start by ",
    )
    if stripped.startswith(chatter_prefixes):
        evidence_markers = ("## findings", "root cause", "severity", "evidence", "confirmed", "file:", "path:")
        if not any(marker in stripped for marker in evidence_markers):
            return True
    chatter_lines = [
        line.strip()
        for line in stripped.splitlines()
        if line.strip()
    ]
    if chatter_lines:
        chatter_hits = sum(
            1
            for line in chatter_lines[:8]
            if line.startswith(("i will ", "i'll ", "let me ", "now let me "))
        )
        if chatter_hits >= 2 and not any(
            marker in stripped
            for marker in ("## findings", "root cause", "severity", "evidence", "confirmed", "grep ", "read ", "file:", "path:")
        ):
            return True
    return False


def output_has_useful_progress(output_text: str, stderr_text: str, min_output_bytes: int) -> bool:
    combined = f"{output_text}\n{stderr_text}".strip()
    if not combined:
        return False
    normalized = combined.casefold()
    if looks_like_planning_chatter(normalized):
        return False
    return structured_evidence_score(normalized) >= 1


def structured_evidence_score(normalized_text: str) -> int:
    score = 0
    if any(marker in normalized_text for marker in ("## findings", "findings report", "root cause")):
        score += 1
    if any(marker in normalized_text for marker in ("severity", "evidence", "confirmed", "impact:", "recommended")):
        score += 1
    if any(marker in normalized_text for marker in ("file:", "path:", "location:", "`src/", "`lib/", "`docs/")):
        score += 1
    bullet_hits = normalized_text.count("\n- ") + normalized_text.count("\n1. ") + normalized_text.count("\n2. ")
    if bullet_hits >= 2:
        score += 1
    return score


def write_manifest(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def manifest_active_subagents(launches: dict[str, dict[str, Any]]) -> list[dict[str, Any]]:
    items: list[dict[str, Any]] = []
    for subagent_name, launch in sorted(launches.items()):
        items.append(
            {
                "subagent": subagent_name,
                "dispatch_mode": launch.get("dispatch_mode"),
                "runtime_extension_applied": bool(launch.get("runtime_extension_applied", False)),
                "effective_runtime_seconds": int(launch.get("effective_runtime_seconds", launch.get("max_runtime_seconds", 0)) or 0),
                "useful_progress": bool(launch.get("useful_progress", False)),
                "first_useful_output_ms": launch.get("first_useful_output_ms"),
                "startup_timeout_seconds": int(launch.get("startup_timeout_seconds", 0) or 0),
                "no_output_timeout_seconds": int(launch.get("no_output_timeout_seconds", 0) or 0),
                "progress_idle_timeout_seconds": int(launch.get("progress_idle_timeout_seconds", 0) or 0),
                "idle_seconds": int(launch.get("last_idle_seconds", 0) or 0),
            }
        )
    return items


def subagent_result_payload(
    *,
    task_id: str,
    task_class: str,
    subagent_name: str,
    selected_model: str | None,
    subagent_cfg: dict[str, Any],
    dispatch_mode: str,
    risk_class: str,
    domain_tags: list[str],
    max_runtime_seconds: int,
    min_output_bytes: int,
    output_file: Path,
    stderr_path: Path,
    workdir: Path,
    prompt_file: Path,
    route: dict[str, Any],
    run_id: str,
    ts_start: str,
    started: float,
    status: str,
    exit_code: int,
    error_text: str,
) -> dict[str, Any]:
    duration_ms = int((time.monotonic() - started) * 1000)
    prompt_text = read_prompt(prompt_file)
    output_text = load_output_text(output_file)
    stderr_text = load_output_text(stderr_path)
    worker_packet_errors = list(worker_packet_gate.validate_packet_text(prompt_text))
    worker_packet_valid = not worker_packet_errors
    merge_ready = False
    worker_output_valid = status == "success"
    worker_output_errors: list[str] = []
    machine_readable_contract_required = False
    if status == "success":
        merge_ready, worker_output_valid, worker_output_errors, machine_readable_contract_required = packet_merge_contract(
            prompt_text,
            output_text,
            min_output_bytes,
        )
    else:
        machine_readable_contract_required = worker_packet_gate.machine_readable_contract_required(prompt_text)
    contract_failure = "worker packet validation failed" in error_text.casefold() or "worker output contract invalid" in error_text.casefold()
    useful_progress = False if contract_failure else output_has_useful_progress(output_text, stderr_text, min_output_bytes)
    if machine_readable_contract_required and worker_output_valid:
        useful_progress = True
    chatter_only = (not merge_ready) and (not worker_output_valid) and looks_like_planning_chatter(output_text.casefold())
    review_state = review_state_for(status, merge_ready, risk_class)
    target_review_state = subagent_system.target_review_state_for(risk_class)
    target_manifest_review_state = subagent_system.target_manifest_review_state_for(risk_class)
    availability = subagent_availability_signal(status, error_text, output_text, stderr_text)
    route_policy = route_policy_payload(task_id, route, subagent_name, subagent_cfg, dispatch_mode)
    payload = {
        "ts": now_utc(),
        "type": "subagent_run",
        "run_id": run_id,
        "task_id": task_id,
        "task_class": task_class,
        "dispatch_mode": dispatch_mode,
        "subagent": subagent_name,
        "selected_model": selected_model,
        "billing_tier": subagent_cfg.get("billing_tier", "unknown"),
        "speed_tier": subagent_cfg.get("speed_tier", "unknown"),
        "quality_tier": subagent_cfg.get("quality_tier", "unknown"),
        "specialties": subagent_cfg.get("specialties", []),
        "write_scope": subagent_cfg.get("write_scope", "none"),
        "ts_start": ts_start,
        "ts_end": now_utc(),
        "duration_ms": duration_ms,
        "exit_code": exit_code,
        "status": status,
        "merge_ready": merge_ready,
        "useful_progress": useful_progress,
        "chatter_only": chatter_only,
        "review_state": review_state,
        "risk_class": risk_class,
        "target_review_state": target_review_state,
        "target_manifest_review_state": target_manifest_review_state,
        "domain_tags": domain_tags,
        "max_runtime_seconds": max_runtime_seconds,
        "min_output_bytes": min_output_bytes,
        "startup_timeout_seconds": dispatch_timeout_seconds(subagent_cfg, "startup_timeout_seconds", 45),
        "no_output_timeout_seconds": dispatch_timeout_seconds(subagent_cfg, "no_output_timeout_seconds", 120),
        "progress_idle_timeout_seconds": dispatch_timeout_seconds(subagent_cfg, "progress_idle_timeout_seconds", 90),
        "max_runtime_extension_seconds": dispatch_timeout_seconds(subagent_cfg, "max_runtime_extension_seconds", 90),
        "output_file": str(output_file),
        "stderr_file": str(stderr_path),
        "output_bytes": output_size(output_file),
        "stderr_bytes": output_size(stderr_path),
        "time_to_first_useful_output_ms": launch_time_to_first_useful_output_ms(started, output_file, stderr_path, min_output_bytes),
        "subagent_state": availability["subagent_state"],
        "failure_reason": availability["failure_reason"],
        "cooldown_until": availability["cooldown_until"],
        "probe_required": availability["probe_required"],
        "last_quota_exhausted_at": availability["last_quota_exhausted_at"],
        "workdir": str(workdir),
        "prompt_file": str(prompt_file),
        "route_selected_subagent": route.get("selected_subagent"),
        "verification_gate": route.get("verification_gate"),
        "merge_policy": route.get("merge_policy"),
        "error": error_text,
        "worker_packet_valid": worker_packet_valid,
        "worker_packet_errors": worker_packet_errors,
        "machine_readable_contract_required": machine_readable_contract_required,
        "worker_output_valid": worker_output_valid,
        "worker_output_errors": worker_output_errors,
        "selected_budget_units": route_policy["selected_budget_units"],
        "selected_cost_class": route_policy["selected_cost_class"],
        "route_budget_policy": route_policy["route_budget_policy"],
        "route_budget_max_units": route_policy["route_budget_max_units"],
        "route_budget_max_cost_class": route_policy["route_budget_max_cost_class"],
        "route_estimated_cost_class": route_policy["route_estimated_cost_class"],
        "cheap_lane_attempted": route_policy["cheap_lane_attempted"],
        "bridge_fallback_used": route_policy["bridge_fallback_used"],
        "internal_escalation_used": route_policy["internal_escalation_used"],
        "internal_route_authorized": route_policy["internal_route_authorized"],
        "policy_bypass": route_policy["policy_bypass"],
        "budget_violation": route_policy["budget_violation"],
        "cost_escalation_trigger": route_policy["cost_escalation_trigger"],
        "internal_escalation_receipt": route_policy["internal_escalation_receipt"],
        "route_receipt": route_receipt_payload(route),
    }
    append_jsonl(RUN_LOG_PATH, payload)
    return payload


def subagent_availability_signal(
    status: str,
    error_text: str,
    output_text: str,
    stderr_text: str,
) -> dict[str, Any]:
    combined = "\n".join(
        part for part in [error_text, output_text, stderr_text] if isinstance(part, str) and part.strip()
    ).lower()
    signal = {
        "subagent_state": "active",
        "failure_reason": "",
        "cooldown_until": "",
        "probe_required": False,
        "last_quota_exhausted_at": "",
    }
    if status == "success":
        return signal
    if "policy violation:" in combined or "worker packet validation failed" in combined or "worker output contract invalid" in combined:
        return signal

    if any(
        marker in combined
        for marker in (
            "quota exceeded",
            "quota exhausted",
            "daily quota",
            "daily limit",
            "try again tomorrow",
            "usage limit reached for today",
        )
    ):
        now_ts = subagent_system.now_utc()
        return {
            "subagent_state": "quota_exhausted",
            "failure_reason": "daily_quota_exhausted",
            "cooldown_until": subagent_system.next_utc_day_iso(),
            "probe_required": True,
            "last_quota_exhausted_at": now_ts,
        }
    if any(
        marker in combined
        for marker in (
            "rate limit",
            "too many requests",
            "429",
            "requests per minute",
        )
    ):
        return {
            "subagent_state": "degraded",
            "failure_reason": "rate_limited",
            "cooldown_until": subagent_system.future_utc_iso(minutes=30),
            "probe_required": True,
            "last_quota_exhausted_at": "",
        }
    if any(
        marker in combined
        for marker in (
            "invalid api key",
            "authentication failed",
            "unauthorized",
            "invalid credentials",
            "permission denied",
        )
    ):
        return {
            "subagent_state": "degraded",
            "failure_reason": "auth_invalid",
            "cooldown_until": "",
            "probe_required": True,
            "last_quota_exhausted_at": "",
        }
    if any(
        marker in combined
        for marker in (
            "approval mode",
            "interactive mode",
            "requires interactive",
            "stdin is not a tty",
            "prompt for approval",
        )
    ):
        return {
            "subagent_state": "degraded",
            "failure_reason": "interactive_blocked",
            "cooldown_until": subagent_system.future_utc_iso(hours=12),
            "probe_required": True,
            "last_quota_exhausted_at": "",
        }
    if status == "timeout":
        if "startup timeout without output" in combined:
            return {
                "subagent_state": "degraded",
                "failure_reason": "startup_timeout",
                "cooldown_until": subagent_system.future_utc_iso(minutes=30),
                "probe_required": True,
                "last_quota_exhausted_at": "",
            }
        if "no-output timeout without useful progress" in combined:
            return {
                "subagent_state": "degraded",
                "failure_reason": "no_output_timeout",
                "cooldown_until": subagent_system.future_utc_iso(minutes=30),
                "probe_required": True,
                "last_quota_exhausted_at": "",
            }
        if "stalled after useful progress" in combined:
            return {
                "subagent_state": "degraded",
                "failure_reason": "stalled_after_progress",
                "cooldown_until": subagent_system.future_utc_iso(minutes=20),
                "probe_required": True,
                "last_quota_exhausted_at": "",
            }
        return {
            "subagent_state": "degraded",
            "failure_reason": "runtime_unstable",
            "cooldown_until": subagent_system.future_utc_iso(minutes=30),
            "probe_required": True,
            "last_quota_exhausted_at": "",
        }
    return {
        "subagent_state": "degraded",
        "failure_reason": "runtime_unstable",
        "cooldown_until": "",
        "probe_required": True,
        "last_quota_exhausted_at": "",
    }


def launch_time_to_first_useful_output_ms(
    started: float,
    output_file: Path,
    stderr_path: Path,
    min_output_bytes: int,
) -> int | None:
    output_text = load_output_text(output_file)
    stderr_text = load_output_text(stderr_path)
    if not output_has_useful_progress(output_text, stderr_text, min_output_bytes):
        return None
    return int((time.monotonic() - started) * 1000)


def launch_progress_snapshot(launch: dict[str, Any]) -> dict[str, Any]:
    output_file: Path = launch["output_file"]
    stderr_path: Path = launch["stderr_path"]
    output_bytes = output_size(output_file)
    stderr_bytes = output_size(stderr_path)
    output_text = load_output_text(output_file)
    stderr_text = load_output_text(stderr_path)
    useful_progress = output_has_useful_progress(output_text, stderr_text, int(launch["min_output_bytes"]))
    now_mono = time.monotonic()
    grew = (
        output_bytes > int(launch.get("last_output_bytes", 0))
        or stderr_bytes > int(launch.get("last_stderr_bytes", 0))
    )
    if grew:
        launch["last_output_bytes"] = output_bytes
        launch["last_stderr_bytes"] = stderr_bytes
        launch["last_activity_at"] = now_mono
    if useful_progress and launch.get("first_useful_output_ms") is None:
        launch["first_useful_output_ms"] = int((now_mono - float(launch["started"])) * 1000)
    launch["useful_progress"] = useful_progress
    launch["last_progress_check_at"] = now_mono
    launch["last_idle_seconds"] = int(now_mono - float(launch.get("last_activity_at", launch["started"])))
    return {
        "output_bytes": output_bytes,
        "stderr_bytes": stderr_bytes,
        "useful_progress": useful_progress,
        "grew": grew,
        "observed_output": (output_bytes + stderr_bytes) > 0,
        "first_useful_output_ms": launch.get("first_useful_output_ms"),
        "idle_seconds": int(now_mono - float(launch.get("last_activity_at", launch["started"]))),
    }


def runtime_extension_seconds(launch: dict[str, Any]) -> int:
    base_limit = int(launch["max_runtime_seconds"])
    quality_tier = str(launch["subagent_cfg"].get("quality_tier", "medium"))
    if quality_tier == "high":
        extension = min(90, max(30, base_limit // 2))
    else:
        extension = min(60, max(20, base_limit // 3))
    cap = int(launch.get("max_runtime_extension_seconds", 0) or 0)
    if cap > 0:
        extension = min(extension, cap)
    return extension


def run_subagent(
    task_id: str,
    task_class: str,
    subagent_name: str,
    prompt_file: Path,
    output_file: Path,
    workdir: Path,
    route: dict[str, Any],
    subagent_cfg: dict[str, Any],
    dispatch_mode: str,
) -> dict[str, Any]:
    policy_violation = dispatch_policy_violation(task_id, route, subagent_name, dispatch_mode, subagent_cfg)
    if policy_violation:
        stderr_path = output_file.with_suffix(output_file.suffix + ".stderr.log")
        try:
            stderr_path.parent.mkdir(parents=True, exist_ok=True)
            stderr_path.write_text(policy_violation + "\n", encoding="utf-8")
        except OSError:
            pass
        return subagent_result_payload(
            task_id=task_id,
            task_class=task_class,
            subagent_name=subagent_name,
            selected_model=selected_model_for_subagent(route, subagent_name, subagent_cfg),
            subagent_cfg=subagent_cfg,
            dispatch_mode=dispatch_mode,
            risk_class=route_risk_class(route),
            domain_tags=infer_domain_tags(read_prompt(prompt_file), task_class),
            max_runtime_seconds=dispatch_runtime_limit(
                route_runtime_limit(route_subagent_item(route, subagent_name) or route, subagent_cfg),
                dispatch_mode,
                route_subagent_item(route, subagent_name) or route,
                subagent_cfg,
            ),
            min_output_bytes=route_min_output_bytes(route, subagent_cfg),
            output_file=output_file,
            stderr_path=stderr_path,
            workdir=workdir,
            prompt_file=prompt_file,
            route=route,
            run_id=f"spr-{uuid.uuid4().hex[:12]}",
            ts_start=now_utc(),
            started=time.monotonic(),
            status="failure",
            exit_code=2,
            error_text=f"policy violation: {policy_violation}",
        )

    launch = start_subagent_process(
        task_id,
        task_class,
        subagent_name,
        prompt_file,
        output_file,
        workdir,
        route,
        subagent_cfg,
        dispatch_mode,
    )
    if "result" in launch:
        return launch["result"]

    while True:
        process: subprocess.Popen[str] = launch["process"]
        progress = launch_progress_snapshot(launch)
        elapsed = time.monotonic() - float(launch["started"])
        effective_runtime_seconds = int(launch.get("effective_runtime_seconds", launch["max_runtime_seconds"]))
        if (
            process.poll() is None
            and not progress.get("observed_output")
            and elapsed > int(launch.get("startup_timeout_seconds", 45))
        ):
            return terminate_subagent_process(
                launch,
                f"cli subagent hit startup timeout without output ({launch.get('startup_timeout_seconds', 45)}s)",
                status_override="timeout",
                exit_code_override=124,
            )
        if (
            process.poll() is None
            and not progress.get("useful_progress")
            and int(progress.get("idle_seconds", 0)) > int(launch.get("no_output_timeout_seconds", 120))
        ):
            return terminate_subagent_process(
                launch,
                f"cli subagent hit no-output timeout without useful progress ({launch.get('no_output_timeout_seconds', 120)}s)",
                status_override="timeout",
                exit_code_override=124,
            )
        if (
            process.poll() is None
            and elapsed > effective_runtime_seconds
            and launch.get("runtime_extension_applied") is not True
            and progress.get("useful_progress")
            and int(progress.get("idle_seconds", 0)) <= 45
        ):
            extension = runtime_extension_seconds(launch)
            launch["runtime_extension_applied"] = True
            launch["effective_runtime_seconds"] = effective_runtime_seconds + extension
            time.sleep(0.5)
            continue
        if process.poll() is None and elapsed > int(launch.get("effective_runtime_seconds", launch["max_runtime_seconds"])):
            return terminate_subagent_process(
                launch,
                f"cli subagent exceeded runtime limit ({launch.get('effective_runtime_seconds', launch['max_runtime_seconds'])}s)",
                status_override="timeout",
                exit_code_override=124,
            )
        if (
            process.poll() is None
            and progress.get("useful_progress")
            and int(progress.get("idle_seconds", 0)) > int(launch.get("progress_idle_timeout_seconds", 90))
        ):
            return terminate_subagent_process(
                launch,
                f"cli subagent stalled after useful progress ({launch.get('progress_idle_timeout_seconds', 90)}s idle)",
                status_override="timeout",
                exit_code_override=124,
            )
        if process.poll() is not None:
            return finalize_subagent_process(launch)
        time.sleep(0.5)


def start_subagent_process(
    task_id: str,
    task_class: str,
    subagent_name: str,
    prompt_file: Path,
    output_file: Path,
    workdir: Path,
    route: dict[str, Any],
    subagent_cfg: dict[str, Any],
    dispatch_mode: str,
) -> dict[str, Any]:
    ensure_dirs()
    output_file.parent.mkdir(parents=True, exist_ok=True)
    stderr_path = output_file.with_suffix(output_file.suffix + ".stderr.log")
    prompt = read_prompt(prompt_file)
    selected_model = selected_model_for_subagent(route, subagent_name, subagent_cfg)
    subagent_route = route_subagent_item(route, subagent_name)
    domain_tags = infer_domain_tags(prompt, task_class)
    risk_class = route_risk_class(route)
    max_runtime_seconds = dispatch_runtime_limit(
        route_runtime_limit(subagent_route or route, subagent_cfg),
        dispatch_mode,
        subagent_route or route,
        subagent_cfg,
    )
    min_output_bytes = route_min_output_bytes(route, subagent_cfg)
    startup_timeout_seconds = dispatch_timeout_seconds(subagent_cfg, "startup_timeout_seconds", 45)
    no_output_timeout_seconds = dispatch_timeout_seconds(subagent_cfg, "no_output_timeout_seconds", 120)
    progress_idle_timeout_seconds = dispatch_timeout_seconds(subagent_cfg, "progress_idle_timeout_seconds", 90)
    max_runtime_extension_seconds = dispatch_timeout_seconds(subagent_cfg, "max_runtime_extension_seconds", 90)
    run_id = f"spr-{uuid.uuid4().hex[:12]}"
    ts_start = now_utc()
    started = time.monotonic()

    worker_packet_errors = list(worker_packet_gate.validate_packet_text(prompt))
    if worker_packet_errors:
        error_text = f"worker packet validation failed: {'; '.join(worker_packet_errors)}"
        stderr_path.write_text(error_text + "\n", encoding="utf-8")
        return {
            "result": subagent_result_payload(
                task_id=task_id,
                task_class=task_class,
                subagent_name=subagent_name,
                selected_model=selected_model,
                subagent_cfg=subagent_cfg,
                dispatch_mode=dispatch_mode,
                risk_class=risk_class,
                domain_tags=domain_tags,
                max_runtime_seconds=max_runtime_seconds,
                min_output_bytes=min_output_bytes,
                output_file=output_file,
                stderr_path=stderr_path,
                workdir=workdir,
                prompt_file=prompt_file,
                route=route,
                run_id=run_id,
                ts_start=ts_start,
                started=started,
                status="failure",
                exit_code=2,
                error_text=error_text,
            )
        }

    if not subagent_supports_dispatch(subagent_cfg):
        error_text = f"cli subagent dispatch unavailable for {subagent_name}; internal/senior lanes require orchestrator-owned handling"
        stderr_path.write_text(error_text + "\n", encoding="utf-8")
        return {
            "result": subagent_result_payload(
                task_id=task_id,
                task_class=task_class,
                subagent_name=subagent_name,
                selected_model=selected_model,
                subagent_cfg=subagent_cfg,
                dispatch_mode=dispatch_mode,
                risk_class=risk_class,
                domain_tags=domain_tags,
                max_runtime_seconds=max_runtime_seconds,
                min_output_bytes=min_output_bytes,
                output_file=output_file,
                stderr_path=stderr_path,
                workdir=workdir,
                prompt_file=prompt_file,
                route=route,
                run_id=run_id,
                ts_start=ts_start,
                started=started,
                status="failure",
                exit_code=1,
                error_text=error_text,
            )
        }

    try:
        cmd, use_stdout_output = subagent_command(
            subagent_name,
            prompt,
            output_file,
            workdir,
            selected_model,
            subagent_cfg,
            route,
        )
        stderr_handle = stderr_path.open("w", encoding="utf-8")
        stdout_handle = output_file.open("w", encoding="utf-8") if use_stdout_output else None
        process = subprocess.Popen(
            cmd,
            cwd=str(workdir),
            stdout=stdout_handle if stdout_handle is not None else stderr_handle,
            stderr=stderr_handle,
            text=True,
            env={**os.environ.copy(), **subagent_env(subagent_cfg)},
        )
        return {
            "process": process,
            "stdout_handle": stdout_handle,
            "stderr_handle": stderr_handle,
            "task_id": task_id,
            "task_class": task_class,
            "subagent_name": subagent_name,
            "prompt_file": prompt_file,
            "output_file": output_file,
            "stderr_path": stderr_path,
            "workdir": workdir,
            "route": route,
            "subagent_cfg": subagent_cfg,
            "dispatch_mode": dispatch_mode,
            "selected_model": selected_model,
            "domain_tags": domain_tags,
            "risk_class": risk_class,
            "max_runtime_seconds": max_runtime_seconds,
            "min_output_bytes": min_output_bytes,
            "startup_timeout_seconds": startup_timeout_seconds,
            "no_output_timeout_seconds": no_output_timeout_seconds,
            "progress_idle_timeout_seconds": progress_idle_timeout_seconds,
            "max_runtime_extension_seconds": max_runtime_extension_seconds,
            "run_id": run_id,
            "ts_start": ts_start,
            "started": started,
            "last_output_bytes": 0,
            "last_stderr_bytes": 0,
            "last_activity_at": started,
            "last_progress_check_at": started,
            "first_useful_output_ms": None,
            "useful_progress": False,
            "runtime_extension_applied": False,
            "effective_runtime_seconds": max_runtime_seconds,
        }
    except Exception as exc:
        try:
            stderr_path.write_text(str(exc) + "\n", encoding="utf-8")
        except OSError:
            pass
        return {
            "result": subagent_result_payload(
                task_id=task_id,
                task_class=task_class,
                subagent_name=subagent_name,
                selected_model=selected_model,
                subagent_cfg=subagent_cfg,
                dispatch_mode=dispatch_mode,
                risk_class=risk_class,
                domain_tags=domain_tags,
                max_runtime_seconds=max_runtime_seconds,
                min_output_bytes=min_output_bytes,
                output_file=output_file,
                stderr_path=stderr_path,
                workdir=workdir,
                prompt_file=prompt_file,
                route=route,
                run_id=run_id,
                ts_start=ts_start,
                started=started,
                status="failure",
                exit_code=1,
                error_text=str(exc),
            )
        }


def close_launch_handles(launch: dict[str, Any]) -> None:
    stdout_handle = launch.get("stdout_handle")
    stderr_handle = launch.get("stderr_handle")
    if stdout_handle is not None:
        stdout_handle.close()
    if stderr_handle is not None:
        stderr_handle.close()


def finalize_subagent_process(
    launch: dict[str, Any],
    *,
    status_override: str | None = None,
    exit_code_override: int | None = None,
    error_text: str = "",
) -> dict[str, Any]:
    process: subprocess.Popen[str] = launch["process"]
    if process.poll() is None:
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)
    close_launch_handles(launch)
    exit_code = int(process.returncode if process.returncode is not None else 1)
    status = status_override or "success"
    if exit_code_override is not None:
        exit_code = exit_code_override
    elif status_override is None and (exit_code != 0 or output_size(launch["output_file"]) == 0):
        status = "failure"
    return subagent_result_payload(
        task_id=launch["task_id"],
        task_class=launch["task_class"],
        subagent_name=launch["subagent_name"],
        selected_model=launch["selected_model"],
        subagent_cfg=launch["subagent_cfg"],
        dispatch_mode=launch["dispatch_mode"],
        risk_class=launch["risk_class"],
        domain_tags=launch["domain_tags"],
        max_runtime_seconds=launch["max_runtime_seconds"],
        min_output_bytes=launch["min_output_bytes"],
        output_file=launch["output_file"],
        stderr_path=launch["stderr_path"],
        workdir=launch["workdir"],
        prompt_file=launch["prompt_file"],
        route=launch["route"],
        run_id=launch["run_id"],
        ts_start=launch["ts_start"],
        started=launch["started"],
        status=status,
        exit_code=exit_code,
        error_text=error_text,
    )


def terminate_subagent_process(
    launch: dict[str, Any],
    reason: str,
    *,
    status_override: str = "terminated",
    exit_code_override: int = 143,
) -> dict[str, Any]:
    process: subprocess.Popen[str] = launch["process"]
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)
    return finalize_subagent_process(
        launch,
        status_override=status_override,
        exit_code_override=exit_code_override,
        error_text=reason,
    )


def route_snapshot(task_class: str, task_id: str | None = None) -> tuple[dict[str, Any], dict[str, Any]]:
    snapshot = subagent_system.init_snapshot(task_id)
    route = subagent_system.route_subagent(task_class)
    return snapshot, route


def candidate_subagent_cfg(snapshot: dict[str, Any], subagent_name: str) -> dict[str, Any]:
    subagents = snapshot.get("subagents", {})
    cfg = subagents.get(subagent_name, {})
    if not cfg:
        raise ValueError(f"Cli subagent not found in snapshot: {subagent_name}")
    return cfg


def run_single(argv: list[str]) -> int:
    if len(argv) < 7:
        print(
            "Usage: python3 _vida/scripts/subagent-dispatch.py subagent <task_id> <task_class> <subagent> <prompt_file> <output_file> [workdir]",
            file=sys.stderr,
        )
        return 1
    task_id, task_class, subagent_name, prompt_file_raw, output_file_raw = argv[2:7]
    workdir = Path(argv[7]).resolve() if len(argv) > 7 else ROOT_DIR
    prompt_file = Path(prompt_file_raw).resolve()
    output_file = Path(output_file_raw).resolve()
    snapshot, route = route_snapshot(task_class, task_id)
    payload = run_subagent(
        task_id,
        task_class,
        subagent_name,
        prompt_file,
        output_file,
        workdir,
        route,
        candidate_subagent_cfg(snapshot, subagent_name),
        "single",
    )
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0 if payload["status"] == "success" else 2


def ensemble_subagents(route: dict[str, Any]) -> list[str]:
    subagents = list(route.get("fanout_subagents", []))
    primary = route.get("selected_subagent")
    if primary and primary not in subagents:
        subagents.insert(0, primary)
    if not subagents and primary:
        subagents = [primary]
    deduped: list[str] = []
    seen: set[str] = set()
    for subagent_name in subagents:
        if subagent_name in seen:
            continue
        deduped.append(subagent_name)
        seen.add(subagent_name)
    return deduped


def load_output_text(path: Path) -> str:
    if not path.exists():
        return ""
    try:
        return path.read_text(encoding="utf-8", errors="ignore").strip()
    except OSError:
        return ""


def build_issue_contract_from_analysis_manifest(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
    analysis_manifest: dict[str, Any],
) -> tuple[dict[str, Any], str]:
    results = analysis_manifest.get("results", [])
    if not isinstance(results, list):
        return {}, "missing_issue_contract"

    candidates: list[dict[str, Any]] = []
    for item in results:
        if not isinstance(item, dict) or item.get("status") != "success":
            continue
        output_file_raw = str(item.get("output_file", "")).strip()
        if not output_file_raw:
            continue
        output_file = Path(output_file_raw)
        payload = worker_packet_gate.extract_json_payload(load_output_text(output_file)) or {}
        raw_issue_contract = payload.get("issue_contract")
        if not isinstance(raw_issue_contract, dict):
            continue
        candidates.append(
            normalized_issue_contract(
                task_id,
                task_class,
                route,
                raw_issue_contract,
                source_subagents=[str(item.get("subagent", "")).strip()],
                source_output_files=[str(output_file)],
                evidence_refs=normalized_string_list(payload.get("evidence_refs")),
            )
        )

    if not candidates:
        return {}, "missing_issue_contract"

    statuses = [policy_value(item.get("status"), "insufficient_evidence") for item in candidates]
    priority = {
        "spec_delta_required": 0,
        "issue_closed_no_fix": 1,
        "insufficient_evidence": 2,
        "writer_ready": 3,
    }
    selected = min(candidates, key=lambda item: priority.get(policy_value(item.get("status"), "insufficient_evidence"), 2)).copy()
    selected["source_subagents"] = deduped_strings(
        [subagent for item in candidates for subagent in normalized_string_list(item.get("source_subagents"))]
    )
    selected["source_output_files"] = deduped_strings(
        [path for item in candidates for path in normalized_string_list(item.get("source_output_files"))]
    )
    selected["evidence_refs"] = deduped_strings(
        [ref for item in candidates for ref in normalized_string_list(item.get("evidence_refs"))]
    )
    selected["conflict_statuses"] = deduped_strings(statuses)
    if len(set(statuses)) > 1 and selected.get("status") == "writer_ready":
        selected["status"] = "insufficient_evidence"
        selected["resolution_path"] = issue_contract_resolution_path("insufficient_evidence")
    return selected, ""


def digest_text(text: str) -> str:
    if not text:
        return ""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


SEMANTIC_STOPWORDS = {
    "the", "and", "for", "with", "that", "this", "from", "into", "then", "than", "have", "has", "had",
    "still", "they", "them", "their", "there", "here", "were", "was", "are", "not", "but", "can", "could",
    "would", "should", "into", "onto", "after", "before", "under", "over", "more", "most", "very", "also",
    "line", "lines", "file", "files", "path", "paths", "issue", "issues", "root", "cause", "causes",
    "problem", "findings", "report", "analysis", "severity", "confirmed", "evidence", "recommended",
    "fix", "fixes", "high", "medium", "low", "critical", "status", "incomplete", "query", "error", "errors",
    "project", "framework", "worker", "orchestrator", "scope", "verification", "command", "commands",
}


def normalize_plain_text(text: str) -> str:
    if not text:
        return ""
    text = re.sub(r"(?is)<prunable-tools>.*?</prunable-tools>", " ", text)
    text = re.sub(r"(?im)^thinking mode:\s*[^\n]*$", " ", text)
    text = re.sub(r"(?im)^tokens used\s*$", " ", text)
    text = re.sub(r"(?im)^\d[\d,]*\s*$", " ", text)
    text = re.sub(r"(?is)^.*?\b(findings|root causes|confirmed findings|confirmed facts|risks|recommended fixes)\b", r"\1", text, count=1)
    text = re.sub(r"```.*?```", " ", text, flags=re.DOTALL)
    normalized_lines: list[str] = []
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        lowered = line.casefold()
        if lowered.startswith(("exec", "thinking", "codex", "mcp:", "user")):
            continue
        if "succeeded in " in lowered or "exited " in lowered:
            continue
        if lowered.startswith(("0:", "1:", "2:", "3:", "4:", "5:")) and ("read " in lowered or "grep " in lowered):
            continue
        line = re.sub(r"^\s{0,3}(?:[#>*-]+|\d+[.)])\s*", "", line)
        line = re.sub(r"`+", "", line)
        line = line.casefold()
        line = re.sub(r"[^a-z0-9\s:/._-]+", " ", line)
        line = re.sub(r"(?<![/:])[\.,;:!?]+$", "", line)
        line = re.sub(r"\s+", " ", line).strip()
        if len(line) >= 12:
            normalized_lines.append(line)
    return "\n".join(sorted(dict.fromkeys(normalized_lines)))


def canonicalize_json_like(value: Any) -> Any:
    if isinstance(value, dict):
        return {str(key): canonicalize_json_like(value[key]) for key in sorted(value)}
    if isinstance(value, list):
        normalized_items = [canonicalize_json_like(item) for item in value]
        return sorted(normalized_items, key=lambda item: json.dumps(item, sort_keys=True, ensure_ascii=True))
    if isinstance(value, str):
        return normalize_plain_text(value)
    return value


def normalize_output_text(text: str) -> str:
    if not text:
        return ""
    stripped = text.strip()
    try:
        parsed = json.loads(stripped)
    except json.JSONDecodeError:
        return normalize_plain_text(stripped)
    canonical = canonicalize_json_like(parsed)
    return json.dumps(canonical, sort_keys=True, ensure_ascii=True, separators=(",", ":"))


def semantic_anchor_tokens(text: str) -> set[str]:
    normalized = normalize_output_text(text)
    if not normalized:
        return set()
    path_tokens = {
        part
        for part in re.findall(r"[a-z0-9_./-]+", normalized)
        if "/" in part
        for part in re.split(r"[/._-]+", part)
        if len(part) >= 4
    }
    word_tokens = {
        token
        for token in re.findall(r"[a-z0-9_]{4,}", normalized)
        if token not in SEMANTIC_STOPWORDS and not token.isdigit()
    }
    preferred = {
        token
        for token in (path_tokens | word_tokens)
        if token in {
            "json2", "jsonrpc", "adapter", "interceptor", "unread", "message", "messages",
            "dashboard", "menus", "menu", "session", "expired", "access", "denied",
            "auth", "server", "searchread", "search", "load", "query", "counter",
            "repository", "quick", "stats", "navigation", "recovery", "api",
        }
    }
    if preferred:
        return preferred
    return set(sorted(path_tokens | word_tokens)[:24])


def semantic_similarity(left: set[str], right: set[str]) -> float:
    if not left or not right:
        return 0.0
    intersection = len(left & right)
    union = len(left | right)
    return intersection / union if union else 0.0


def preview_text(text: str, limit: int = 160) -> str:
    collapsed = re.sub(r"\s+", " ", text).strip()
    if len(collapsed) <= limit:
        return collapsed
    return collapsed[: limit - 3] + "..."


def cluster_payload(group_key: str, subagents: list[str], preview: str, weight: int = 0) -> dict[str, Any]:
    return {
        "cluster_id": group_key[:12] if group_key else "empty",
        "subagents": sorted(subagents),
        "sample": preview,
        "weight": weight,
    }


def source_backing_weight(text: str) -> int:
    if not text:
        return 0
    lowered = text.casefold()
    weight = 0
    weight += min(5, lowered.count("confirmed")) * 6
    weight += min(5, lowered.count("evidence")) * 5
    weight += min(5, lowered.count("live")) * 4
    weight += min(10, lowered.count("file:")) * 3
    weight += min(10, lowered.count("path:")) * 2
    weight += min(6, lowered.count("_temp/")) * 4
    weight += min(6, lowered.count("docs/")) * 2
    weight += min(6, lowered.count("cite")) * 4
    if "assumption" in lowered:
        weight -= 6
    if "require live validation" in lowered or "need live validation" in lowered:
        weight -= 4
    return max(0, weight)


def build_merge_summary(
    results: list[dict[str, Any]],
    merge_policy: str,
    min_results: int,
    subagent_scores: dict[str, int] | None = None,
) -> dict[str, Any]:
    success_items = [item for item in results if item.get("status") == "success" and item.get("merge_ready") is True]
    failure_subagents = sorted(item["subagent"] for item in results if item.get("status") != "success")
    non_merge_ready_subagents = sorted(
        item["subagent"]
        for item in results
        if item.get("status") == "success" and item.get("merge_ready") is not True
    )
    required_results = max(1, min_results)
    subagent_scores = subagent_scores or {}
    exact_groups: dict[str, list[str]] = {}
    semantic_groups: dict[str, list[str]] = {}
    semantic_previews: dict[str, str] = {}
    semantic_token_index: dict[str, set[str]] = {}
    subagent_evidence_weights: dict[str, int] = {}
    for item in success_items:
        text = load_output_text(Path(item["output_file"]))
        subagent_evidence_weights[item["subagent"]] = source_backing_weight(text)
        exact_text = text if text else f"empty:{item['subagent']}"
        exact_digest = digest_text(exact_text)
        exact_groups.setdefault(exact_digest, []).append(item["subagent"])

        semantic_text = normalize_output_text(text)
        if not semantic_text:
            semantic_text = f"empty:{item['subagent']}"
        semantic_tokens = semantic_anchor_tokens(text)
        semantic_digest = ""
        for existing_key, existing_tokens in semantic_token_index.items():
            if semantic_similarity(semantic_tokens, existing_tokens) >= 0.33:
                semantic_digest = existing_key
                semantic_token_index[existing_key] = existing_tokens | semantic_tokens
                break
        if not semantic_digest:
            semantic_digest = digest_text(" ".join(sorted(semantic_tokens)) or semantic_text)
            semantic_token_index[semantic_digest] = set(semantic_tokens)
        semantic_groups.setdefault(semantic_digest, []).append(item["subagent"])
        existing_preview = semantic_previews.get(semantic_digest, "")
        candidate_preview = preview_text(semantic_text)
        semantic_previews[semantic_digest] = existing_preview if len(existing_preview) >= len(candidate_preview) else candidate_preview

    exact_agreements = sorted(sorted(group) for group in exact_groups.values() if len(group) > 1)
    semantic_weight_index = {
        group_key: sum(
            int(subagent_scores.get(subagent_name, 0)) + int(subagent_evidence_weights.get(subagent_name, 0))
            for subagent_name in subagents
        )
        for group_key, subagents in semantic_groups.items()
    }
    semantic_clusters = sorted(
        (
            cluster_payload(
                group_key,
                subagents,
                semantic_previews.get(group_key, ""),
                semantic_weight_index.get(group_key, 0),
            )
            for group_key, subagents in semantic_groups.items()
        ),
        key=lambda item: (
            -len(item["subagents"]),
            -int(item.get("weight", 0)),
            item["subagents"],
        ),
    )
    semantic_agreements = [cluster for cluster in semantic_clusters if len(cluster["subagents"]) > 1]
    unique_findings = [cluster for cluster in semantic_clusters if len(cluster["subagents"]) == 1]
    cluster_weights = {cluster["cluster_id"]: int(cluster.get("weight", 0)) for cluster in semantic_clusters}

    exact_consensus = len(success_items) > 1 and len(exact_groups) == 1
    semantic_consensus = len(success_items) > 1 and len(semantic_groups) == 1
    largest_cluster = semantic_clusters[0] if semantic_clusters else None
    second_cluster_size = len(semantic_clusters[1]["subagents"]) if len(semantic_clusters) > 1 else 0
    dominant_weight = cluster_weights.get(largest_cluster["cluster_id"], 0) if largest_cluster else 0
    second_weight = cluster_weights.get(semantic_clusters[1]["cluster_id"], 0) if len(semantic_clusters) > 1 else 0
    semantic_majority = (
        largest_cluster is not None
        and len(largest_cluster["subagents"]) >= required_results
        and len(largest_cluster["subagents"]) > second_cluster_size
        and len(semantic_groups) > 1
    )
    weighted_semantic_majority = (
        largest_cluster is not None
        and len(semantic_groups) > 1
        and dominant_weight >= max(60, required_results * 20)
        and dominant_weight > second_weight
        and (second_weight == 0 or dominant_weight >= int(second_weight * 1.25))
    )
    decision_ready = (
        largest_cluster is not None
        and weighted_semantic_majority
        and len(success_items) >= max(2, required_results - 1)
        and dominant_weight >= max(90, required_results * 30)
    )
    consensus_mode = "none"
    if exact_consensus:
        consensus_mode = "exact"
    elif semantic_consensus:
        consensus_mode = "semantic"
    elif semantic_majority:
        consensus_mode = "semantic_majority"
    elif weighted_semantic_majority:
        consensus_mode = "semantic_weighted_majority"

    open_conflicts: list[dict[str, Any]] = []
    if merge_policy == "consensus_with_conflict_flag" and len(semantic_groups) > 1:
        if consensus_mode in {"semantic_majority", "semantic_weighted_majority"} and largest_cluster is not None:
            open_conflicts = [cluster for cluster in semantic_clusters if cluster != largest_cluster]
        else:
            open_conflicts = semantic_clusters

    if (
        not decision_ready
        and semantic_consensus
        and len(success_items) >= max(2, required_results - 1)
        and len(open_conflicts) == 0
    ):
        decision_ready = True

    tie_break_recommended = False
    tie_break_reason = ""
    if len(success_items) < required_results and not decision_ready:
        tie_break_recommended = True
        tie_break_reason = "fanout_min_results_not_met"
    elif merge_policy == "consensus_with_conflict_flag" and consensus_mode == "none" and len(semantic_groups) > 1:
        tie_break_recommended = True
        tie_break_reason = "semantic_conflict_without_majority"

    return {
        "merge_policy": merge_policy,
        "success_subagents": sorted(item["subagent"] for item in success_items),
        "failure_subagents": failure_subagents,
        "non_merge_ready_subagents": non_merge_ready_subagents,
        "agreements": exact_agreements,
        "semantic_agreements": semantic_agreements,
        "unique_findings": unique_findings,
        "open_conflicts": open_conflicts,
        "distinct_success_outputs": len(exact_groups),
        "distinct_semantic_outputs": len(semantic_groups),
        "exact_consensus": exact_consensus,
        "semantic_consensus": semantic_consensus,
        "semantic_majority": semantic_majority,
        "semantic_weighted_majority": weighted_semantic_majority,
        "decision_ready": decision_ready,
        "consensus_mode": consensus_mode,
        "dominant_finding": largest_cluster,
        "dominant_weight": dominant_weight,
        "second_weight": second_weight,
        "cluster_weights": cluster_weights,
        "subagent_evidence_weights": subagent_evidence_weights,
        "tie_break_recommended": tie_break_recommended,
        "tie_break_reason": tie_break_reason,
        "subagent_exhausted": len(success_items) < required_results,
        "orchestrator_review_required": tie_break_recommended or len(open_conflicts) > 0,
    }


def clone_json_payload(payload: Any) -> Any:
    return json.loads(json.dumps(payload))


def route_subagent_scores(route: dict[str, Any]) -> dict[str, int]:
    scores: dict[str, int] = {}
    primary = route.get("selected_subagent")
    if isinstance(primary, str) and primary:
        scores[primary] = int(route.get("effective_score", 0))
    for item in route.get("fallback_subagents", []):
        subagent_name = item.get("subagent")
        if not isinstance(subagent_name, str) or not subagent_name or subagent_name in scores:
            continue
        scores[subagent_name] = int(item.get("effective_score", 0))
    return scores


def quality_rank(value: Any) -> int:
    if not isinstance(value, str):
        return 0
    return {"low": 1, "medium": 2, "high": 3}.get(value.strip().casefold(), 0)


def role_rank(value: Any) -> int:
    if not isinstance(value, str):
        return 0
    return {"secondary": 1, "primary": 2}.get(value.strip().casefold(), 0)


def arbitration_candidates(
    route: dict[str, Any],
    snapshot: dict[str, Any],
    requested_fanout: list[str],
    results: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    ordered_subagents: list[str] = []
    for subagent_name in [route.get("selected_subagent"), *requested_fanout]:
        if isinstance(subagent_name, str) and subagent_name and subagent_name not in ordered_subagents:
            ordered_subagents.append(subagent_name)
    for item in route.get("fallback_subagents", []):
        subagent_name = item.get("subagent")
        if isinstance(subagent_name, str) and subagent_name and subagent_name not in ordered_subagents:
            ordered_subagents.append(subagent_name)

    scores = route_subagent_scores(route)
    result_by_subagent = {item["subagent"]: item for item in results}
    subagents = snapshot.get("subagents", {})
    candidates: list[dict[str, Any]] = []
    for subagent_name in ordered_subagents:
        subagent_cfg = subagents.get(subagent_name, {})
        if not subagent_supports_dispatch(subagent_cfg):
            continue
        if not subagent_cfg.get("enabled") or not subagent_cfg.get("available"):
            continue
        prior = result_by_subagent.get(subagent_name)
        if prior and prior.get("status") != "success":
            continue
        used = prior is not None
        candidates.append(
            {
                "subagent": subagent_name,
                "used": used,
                "selection_reason": "rerun_best_available_subagent" if used else "unused_supported_subagent",
                "quality_rank": quality_rank(subagent_cfg.get("quality_tier")),
                "role_rank": role_rank(subagent_cfg.get("role")),
                "effective_score": scores.get(subagent_name, 0),
            }
        )

    candidates.sort(
        key=lambda item: (
            1 if item["used"] else 0,
            -int(item["quality_rank"]),
            -int(item["role_rank"]),
            -int(item["effective_score"]),
            str(item["subagent"]),
        )
    )
    return candidates


def arbitration_prompt_text(
    original_prompt: str,
    task_class: str,
    merge_summary: dict[str, Any],
    results: list[dict[str, Any]],
) -> str:
    result_by_subagent = {item["subagent"]: item for item in results if item.get("status") == "success"}
    allowed_cluster_ids = [cluster.get("cluster_id", "") for cluster in merge_summary.get("open_conflicts", [])]
    blocking_question = "Which existing semantic cluster, if any, should the orchestrator treat as the bounded tie-break decision?"
    lines = [
        *worker_packet_lines(blocking_question=blocking_question),
        f"Task: Resolve bounded ensemble conflict for {task_class}.",
        "Mode: READ-ONLY bounded arbitration.",
        "Scope: original prompt plus listed open-conflict clusters only.",
        "Must do:",
        f"- Follow project preflight from {project_preflight_doc()} before analysis/test/build commands.",
        "- Use STC by default for this scoped arbitration.",
        "- Select one existing semantic cluster or return no_decision.",
        "- Do not propose a new answer, do not merge clusters, and do not expand scope.",
        "- Answer the blocking question directly through the JSON result.",
        "Verification:",
        "- Review the original prompt, the listed open-conflict clusters, and the cited success-lane excerpts.",
        "Deliverable:",
        '- Return ONLY valid JSON with this shape: {"decision":"select_cluster|no_decision","selected_cluster_id":"<allowed id or empty>","confidence":"high|medium|low","rationale":"<short reason>"}',
        f"- Allowed cluster_ids: {', '.join(cluster_id for cluster_id in allowed_cluster_ids if cluster_id)}",
        "",
        "Original prompt:",
        "<<<PROMPT",
        original_prompt.strip(),
        "PROMPT",
        "",
        "Conflicting semantic clusters:",
    ]
    for cluster in merge_summary.get("open_conflicts", []):
        cluster_id = str(cluster.get("cluster_id", ""))
        subagents = [str(subagent_name) for subagent_name in cluster.get("subagents", []) if isinstance(subagent_name, str)]
        sample = str(cluster.get("sample", "")).strip()
        lines.append(f"- cluster_id: {cluster_id}")
        lines.append(f"  subagents: {', '.join(subagents) if subagents else '(none)'}")
        lines.append(f"  normalized_sample: {sample or '(empty)'}")
        for subagent_name in subagents:
            payload = result_by_subagent.get(subagent_name)
            if not payload:
                continue
            raw_excerpt = preview_text(load_output_text(Path(payload["output_file"])), 420)
            if raw_excerpt:
                lines.append(f"  {subagent_name}_excerpt: {raw_excerpt}")
        lines.append("")
    return "\n".join(lines).strip() + "\n"


def verification_prompt_text(
    original_prompt: str,
    task_class: str,
    verification_task_class: str,
    merge_summary: dict[str, Any],
    post_arbitration_merge_summary: dict[str, Any],
    results: list[dict[str, Any]],
) -> str:
    effective_summary = post_arbitration_merge_summary or merge_summary
    dominant = effective_summary.get("dominant_finding") or {}
    dominant_cluster = str(dominant.get("cluster_id", "")).strip()
    dominant_sample = str(dominant.get("sample", "")).strip()
    success_subagents = effective_summary.get("success_subagents", [])
    conflict_clusters = effective_summary.get("open_conflicts", [])
    blocking_question = "Is orchestrator synthesis justified from the current primary ensemble result, and if not, what blocker prevents it?"

    lines = [
        *worker_packet_lines(blocking_question=blocking_question),
        f"Task: Verify orchestrator synthesis readiness for {task_class}.",
        "Mode: READ-ONLY independent verification.",
        "Scope: original prompt, primary ensemble summary, success-lane excerpts, and listed conflicts only.",
        "Constraints:",
        f"- Follow project preflight from {project_preflight_doc()} before analysis/test/build commands.",
        "- Use STC by default; use PR-CoT only if a bounded verification trade-off appears inside scope.",
        "- Do not re-solve the task from scratch.",
        "- Validate whether the candidate conclusion is sufficiently supported for orchestrator synthesis.",
        "- Highlight contract risks, residual blockers, and whether synthesis is justified.",
        f"- Primary task class: {task_class}",
        f"- Verification task class: {verification_task_class}",
        "Verification:",
        "- Review the original prompt, the primary ensemble summary, the success-lane excerpts, and any open conflicts.",
        "Deliverable:",
        "- Return the machine-readable summary below.",
        "```json",
        worker_machine_readable_contract(),
        "```",
        "",
        "Original prompt:",
        "<<<PROMPT",
        original_prompt.strip(),
        "PROMPT",
        "",
        "Primary ensemble summary:",
        f"- consensus_mode: {effective_summary.get('consensus_mode', 'none')}",
        f"- decision_ready: {effective_summary.get('decision_ready', False)}",
        f"- dominant_cluster_id: {dominant_cluster or '(none)'}",
        f"- dominant_sample: {dominant_sample or '(none)'}",
        f"- success_subagents: {', '.join(success_subagents) if success_subagents else '(none)'}",
        f"- open_conflicts: {len(conflict_clusters)}",
        "",
        "Success lane excerpts:",
    ]
    for item in results:
        if item.get("status") != "success":
            continue
        excerpt = preview_text(load_output_text(Path(item["output_file"])), 320)
        if not excerpt:
            continue
        lines.append(f"- {item['subagent']}: {excerpt}")
    if conflict_clusters:
        lines.extend(["", "Open conflicts:"])
        for cluster in conflict_clusters:
            cluster_id = str(cluster.get("cluster_id", "")).strip()
            sample = str(cluster.get("sample", "")).strip()
            subagents = ", ".join(cluster.get("subagents", [])) or "(none)"
            lines.append(f"- {cluster_id or '(none)'} | subagents={subagents} | sample={sample or '(empty)'}")
    return "\n".join(lines).strip() + "\n"


def analysis_prompt_text(
    original_prompt: str,
    writer_task_class: str,
    analysis_task_class: str,
) -> str:
    blocking_question = "What confirmed root causes, issue classification, and equivalence decision should guide the writer for this task?"
    lines = [
        *worker_packet_lines(blocking_question=blocking_question),
        f"Task: Analyze the current implementation request for {writer_task_class}.",
        "Mode: READ-ONLY pre-write analysis.",
        "Scope: original prompt/spec plus the current repository files relevant to the requested implementation.",
        "Constraints:",
        f"- Follow project preflight from {project_preflight_doc()} before analysis/test/build commands.",
        "- Use STC by default; use MAR only if a bounded root-cause investigation is necessary inside scope.",
        "- Do not modify files and do not act as the writer.",
        "- Return only evidence-backed findings that help the writer choose the correct root-cause fix order.",
        "- Classify the issue as one of: defect_equivalent, defect_needs_contract_update, feature_delta, as_designed, not_a_bug, insufficient_evidence.",
        "- Decide whether the requested change is an equivalent fix or requires a spec/product-contract delta before any writer pass.",
        f"- Writer task class: {writer_task_class}",
        f"- Analysis task class: {analysis_task_class}",
        "Verification:",
        "- Inspect only the relevant workspace files and run bounded read-only commands when needed.",
        "Deliverable:",
        "- Return the machine-readable summary below.",
        "```json",
        analysis_machine_readable_contract(),
        "```",
        "- Populate `issue_contract` even when the answer is blocked or non-equivalent.",
        "- Use `issue_contract.equivalence_assessment=equivalent_fix` only when the writer can proceed without changing the product/spec contract first.",
        "- For this read-only analysis lane, `merge_ready=yes` means the analysis artifact itself is complete enough for orchestrator synthesis and writer routing; it does not mean implementation code already exists.",
        "- If your evidence-backed `issue_contract` is complete enough to route the writer or block it deterministically, set `merge_ready=yes`; use `merge_ready=no` only when the analysis artifact is still incomplete or ambiguous.",
        "",
        "Original prompt/spec:",
        "<<<PROMPT",
        original_prompt.strip(),
        "PROMPT",
    ]
    return "\n".join(lines).strip() + "\n"


def coach_prompt_text(
    original_prompt: str,
    writer_task_class: str,
    coach_task_class: str,
    *,
    coach_slot: int = 1,
    coach_total: int = 1,
) -> str:
    blocking_question = "Does the current implementation satisfy the spec well enough to proceed, or must it return for rework?"
    lines = [
        *worker_packet_lines(blocking_question=blocking_question),
        f"Task: Coach-review the current implementation for {writer_task_class}.",
        "Mode: READ-ONLY post-write formative review.",
        "Scope: original prompt/spec plus the current workspace implementation state only.",
        "Constraints:",
        f"- Follow project preflight from {project_preflight_doc()} before analysis/test/build commands.",
        "- Review the current implementation against the requested scope and constraints.",
        "- If the implementation should continue to independent verification, approve it.",
        "- If there are concrete gaps, return it for rework with specific guidance.",
        "- Do not mutate files, do not act as the orchestrator, and do not replace the final independent verifier.",
        f"- Writer task class: {writer_task_class}",
        f"- Coach task class: {coach_task_class}",
        f"- Planned coach lanes: {coach_total}",
        "- Review independently; do not assume other coach lanes agree with you.",
        "- Treat readiness for independent verification as an individual coach verdict from your lane; do not wait for other coach lanes to finish before setting your verdict fields.",
        "- Do not use pending parallel coach lanes as a blocker or as a reason to set `merge_ready=no`.",
        "Verification:",
        "- Inspect the implementation in the workspace, run only bounded validation commands when needed, and compare the result to the original request.",
        "- If a local tool is unavailable in your environment, record that in `verification_results` or `notes`, not in `blockers`, unless the missing tool proves a concrete implementation gap.",
        "Deliverable:",
        "- Return the machine-readable summary below.",
        "```json",
        coach_machine_readable_contract(),
        "```",
        "- Use EXACTLY one of these two final states:",
        "  1. Approve for verifier handoff: `coach_decision=approved`, `rework_required=no`, `merge_ready=yes`, `blockers=[]`.",
        "  2. Return for writer rework: `coach_decision=return_for_rework`, `rework_required=yes`, `merge_ready=no`, with one or more concrete implementation gaps in `blockers`.",
        "- Here `merge_ready` means ready to hand off to the NEXT stage (final independent verification), not final task completion.",
        "- Set `coach_decision=approved` and `rework_required=no` only when the implementation is ready for the final independent verifier.",
        "- Set `coach_decision=return_for_rework` and `rework_required=yes` when the implementation must go back to the writer.",
        "- Keep `merge_ready` aligned with the verdict: use `yes` only for approved output that is ready for independent verification, otherwise use `no`.",
        "",
        "Original prompt/spec:",
        "<<<PROMPT",
        original_prompt.strip(),
        "PROMPT",
    ]
    return "\n".join(lines).strip() + "\n"


def fresh_rework_prompt_text(
    original_prompt: str,
    writer_task_class: str,
    coach_decision: dict[str, Any],
    *,
    attempt_count: int,
    max_coach_passes: int,
) -> str:
    blockers = normalized_string_list(coach_decision.get("blockers"))
    evidence_refs = normalized_string_list(coach_decision.get("evidence_refs"))
    verification_results = normalized_string_list(coach_decision.get("verification_results"))
    impact_analysis = normalized_impact_analysis(coach_decision.get("impact_analysis"))
    summary = str(
        coach_decision.get("coach_feedback")
        or coach_decision.get("reason")
        or "coach requested a fresh rewrite pass"
    ).strip()
    recommended_next_action = str(coach_decision.get("recommended_next_action", "")).strip()

    lines = [
        original_prompt.rstrip(),
        "",
        "Fresh Rework Handoff:",
        f"- Writer task class: {writer_task_class}",
        f"- Rework attempt: {attempt_count}",
    ]
    if max_coach_passes > 0:
        lines.append(f"- Max coach passes: {max_coach_passes}")
    lines.extend(
        [
            "- Start a fresh implementation pass from the original prompt/spec above.",
            "- Treat the original prompt/spec as the source of truth.",
            "- Use the coach delta below only as corrective guidance, not as a request to continue the previous attempt in place.",
            "- Re-inspect the current workspace before editing; do not assume the previous implementation structure is still valid.",
            "",
            "Coach Delta:",
            f"- Summary: {summary}",
        ]
    )
    if recommended_next_action:
        lines.append(f"- Required next action: {recommended_next_action}")
    if blockers:
        lines.append("- Blocking gaps:")
        lines.extend(f"  - {item}" for item in blockers)
    if evidence_refs:
        lines.append("- Evidence refs:")
        lines.extend(f"  - {item}" for item in evidence_refs)
    if verification_results:
        lines.append("- Verification results:")
        lines.extend(f"  - {item}" for item in verification_results)
    for label, items in (
        ("Affected scope", impact_analysis.get("affected_scope", [])),
        ("Contract impact", impact_analysis.get("contract_impact", [])),
        ("Follow-up actions", impact_analysis.get("follow_up_actions", [])),
        ("Residual risks", impact_analysis.get("residual_risks", [])),
    ):
        if not items:
            continue
        lines.append(f"- {label}:")
        lines.extend(f"  - {item}" for item in items)
    return "\n".join(lines).strip() + "\n"


def coach_payload_conflict_state(
    requested_decision: str,
    *,
    merge_ready: str,
    rework_required: str,
    blockers: list[str],
) -> tuple[str, list[str]]:
    invalid_reasons: list[str] = []
    if requested_decision == "approved":
        if merge_ready == "no":
            invalid_reasons.append("approved_conflicts_with_merge_ready")
        if rework_required == "yes":
            invalid_reasons.append("approved_conflicts_with_rework_required")
        if blockers:
            invalid_reasons.append("approved_conflicts_with_blockers")
        return ("invalid_coach_payload.approved_conflict", invalid_reasons) if invalid_reasons else ("", [])
    if requested_decision == "return_for_rework":
        if merge_ready == "yes":
            invalid_reasons.append("rework_conflicts_with_merge_ready")
        if rework_required == "no":
            invalid_reasons.append("rework_conflicts_with_rework_required")
        return ("invalid_coach_payload.rework_conflict", invalid_reasons) if invalid_reasons else ("", [])

    positive_signals = 0
    negative_signals = 0
    if merge_ready == "yes":
        positive_signals += 1
    if rework_required == "no":
        positive_signals += 1
    if merge_ready == "no":
        negative_signals += 1
    if rework_required == "yes":
        negative_signals += 1
    if blockers:
        negative_signals += 1
    if positive_signals > 0 and negative_signals > 0:
        return "invalid_coach_payload.ambiguous_finality", ["ambiguous_finality_signals"]
    return "", []


def parse_coach_decision(output_text: str) -> dict[str, Any]:
    payload = worker_packet_gate.extract_json_payload(output_text) or parse_json_object(output_text)
    if not payload:
        return {
            "approved": False,
            "coach_decision": "coach_failed",
            "payload_state": "missing_payload",
            "invalid_reasons": ["missing_coach_decision_payload"],
            "rework_required": "yes",
            "coach_feedback": "",
            "recommended_next_action": "",
            "reason": "missing_coach_decision_payload",
            "parsed_json": False,
            "blockers": [],
            "evidence_refs": [],
            "verification_results": [],
            "impact_analysis": {},
            "answer": "",
            "merge_ready_effective": "no",
            "raw_merge_ready": "",
            "raw_rework_required": "",
        }
    merge_ready = str(payload.get("merge_ready", "")).strip().casefold()
    requested_decision = str(payload.get("coach_decision", "")).strip().casefold()
    rework_required = str(payload.get("rework_required", "")).strip().casefold()
    blockers = normalized_string_list(payload.get("blockers"))
    coach_feedback = str(payload.get("coach_feedback", payload.get("notes", ""))).strip()
    answer = str(payload.get("answer", "")).strip()
    recommended_next_action = str(payload.get("recommended_next_action", "")).strip()
    evidence_refs = normalized_string_list(payload.get("evidence_refs"))
    verification_results = normalized_string_list(payload.get("verification_results"))
    impact_analysis = normalized_impact_analysis(payload.get("impact_analysis"))

    if requested_decision not in {"approved", "return_for_rework"}:
        if rework_required == "yes" or merge_ready == "no" or blockers:
            requested_decision = "return_for_rework"
        elif merge_ready == "yes" or rework_required == "no":
            requested_decision = "approved"
        else:
            requested_decision = ""

    payload_state, invalid_reasons = coach_payload_conflict_state(
        requested_decision,
        merge_ready=merge_ready,
        rework_required=rework_required,
        blockers=blockers,
    )
    if not requested_decision and not invalid_reasons:
        payload_state = "invalid_coach_payload.ambiguous_finality"
        invalid_reasons = ["missing_finality_signals"]
    if invalid_reasons:
        approved = False
        normalized_decision = payload_state
        merge_ready_effective = "no"
        recommended_next_action = recommended_next_action or "rerun_coach_review_with_valid_machine_readable_output"
        reason = "; ".join(invalid_reasons)
        if coach_feedback or answer:
            reason = f"{reason}: {coach_feedback or answer}"
    else:
        approved = requested_decision == "approved" and rework_required != "yes" and merge_ready != "no" and not blockers
        normalized_decision = "approved" if approved else "return_for_rework"
        merge_ready_effective = "yes" if approved else "no"
        reason = ""
        if not approved:
            reason = "; ".join(str(item).strip() for item in blockers if str(item).strip()) or coach_feedback or answer or "coach_return_for_rework"
    return {
        "approved": approved,
        "coach_decision": normalized_decision,
        "payload_state": payload_state or normalized_decision,
        "invalid_reasons": invalid_reasons,
        "rework_required": "no" if approved else "yes",
        "coach_feedback": coach_feedback or answer,
        "recommended_next_action": recommended_next_action,
        "reason": reason,
        "parsed_json": bool(payload),
        "blockers": blockers,
        "evidence_refs": evidence_refs,
        "verification_results": verification_results,
        "impact_analysis": impact_analysis,
        "answer": answer,
        "merge_ready_effective": merge_ready_effective,
        "raw_merge_ready": merge_ready,
        "raw_rework_required": rework_required,
    }


def coach_decision_from_result(result: dict[str, Any]) -> dict[str, Any]:
    output_file = str(result.get("output_file", "")).strip()
    output_path = Path(output_file) if output_file else Path()
    stderr_file = str(result.get("stderr_file", "")).strip()
    stderr_path = Path(stderr_file) if stderr_file else Path()
    status = str(result.get("status", "")).strip()
    candidates = [
        ("output_json_payload", "output_text", load_output_text(output_path) if output_file and output_path.exists() else ""),
        ("stderr_json_payload", "stderr_text", load_output_text(stderr_path) if stderr_file and stderr_path.exists() else ""),
        ("error_text_json_payload", "error_text", str(result.get("error_text", "")).strip()),
        ("status_reason_json_payload", "status_reason", str(result.get("status_reason", "")).strip()),
    ]
    feedback_chain: list[str] = []
    feedback_text = ""
    for json_source, text_source, text in candidates:
        if not text:
            continue
        decision = parse_coach_decision(text)
        if decision.get("parsed_json") is True:
            decision = with_coach_feedback_provenance(decision, primary_source=json_source, source_chain=[json_source])
            decision["subagent"] = str(result.get("subagent", "")).strip()
            decision["output_file"] = output_file
            decision["result_status"] = status
            return decision
        candidate_feedback = coach_feedback_text_candidate(text)
        if candidate_feedback:
            feedback_chain.append(text_source)
            if not feedback_text:
                feedback_text = candidate_feedback

    reason = "missing_coach_decision_payload"
    if feedback_text:
        reason = f"{reason}: {feedback_text}"
    elif status:
        reason = str(result.get("error_text", "")).strip() or str(result.get("status_reason", "")).strip() or status or reason
    decision = {
        "approved": False,
        "coach_decision": "coach_failed",
        "payload_state": "missing_payload",
        "invalid_reasons": ["missing_coach_decision_payload"],
        "rework_required": "yes",
        "coach_feedback": feedback_text,
        "recommended_next_action": "rerun_coach_review_with_valid_machine_readable_output",
        "reason": reason,
        "parsed_json": False,
        "blockers": [],
        "evidence_refs": [],
        "verification_results": [],
        "impact_analysis": {},
        "answer": feedback_text,
        "merge_ready_effective": "no",
        "raw_merge_ready": "",
        "raw_rework_required": "",
    }
    decision = with_coach_feedback_provenance(
        decision,
        primary_source=feedback_chain[0] if feedback_chain else ("error_text" if str(result.get("error_text", "")).strip() else "default_fallback"),
        source_chain=feedback_chain or (["error_text"] if str(result.get("error_text", "")).strip() else ["default_fallback"]),
    )
    decision["subagent"] = str(result.get("subagent", "")).strip()
    decision["output_file"] = output_file
    decision["result_status"] = status
    return decision


def merge_coach_decisions(
    decisions: list[dict[str, Any]],
    *,
    required_results: int,
    merge_policy: str,
) -> dict[str, Any]:
    normalized_required = max(1, int(required_results or 0))
    valid = [decision for decision in decisions if coach_decision_is_valid(decision)]
    valid = valid[: len(valid)]
    selected_subagents = deduped_strings([str(decision.get("subagent", "")).strip() for decision in valid])
    valid_result_count = len(valid)

    if valid_result_count < normalized_required:
        reasons = [f"insufficient_valid_coach_results:{valid_result_count}/{normalized_required}"]
        for decision in decisions:
            reasons.extend(normalized_string_list(decision.get("invalid_reasons")))
        invalid_reasons = deduped_strings(reasons)
        return {
            "approved": False,
            "coach_decision": "coach_failed",
            "payload_state": "coach_failed",
            "invalid_reasons": invalid_reasons,
            "rework_required": "yes",
            "coach_feedback": "",
            "recommended_next_action": "rerun_coach_review_with_independent_valid_outputs",
            "reason": "; ".join(invalid_reasons),
            "parsed_json": False,
            "blockers": [],
            "evidence_refs": deduped_strings(
                [ref for decision in valid for ref in normalized_string_list(decision.get("evidence_refs"))]
            ),
            "verification_results": deduped_strings(
                [item for decision in valid for item in normalized_string_list(decision.get("verification_results"))]
            ),
            "impact_analysis": merge_impact_analyses(valid),
            "answer": "",
            "merge_ready_effective": "no",
            "raw_merge_ready": "",
            "raw_rework_required": "",
            "selected_subagents": selected_subagents,
            "valid_result_count": valid_result_count,
            "required_result_count": normalized_required,
            "merge_policy": merge_policy,
            "coach_results": decisions,
        }

    rework_decisions = [decision for decision in valid if decision.get("coach_decision") == "return_for_rework" or not decision.get("approved")]
    if merge_policy == "unanimous_approve_rework_bias" and not rework_decisions and all(decision.get("approved") is True for decision in valid):
        return {
            "approved": True,
            "coach_decision": "approved",
            "payload_state": "approved",
            "invalid_reasons": [],
            "rework_required": "no",
            "coach_feedback": coach_feedback_summary(valid),
            "recommended_next_action": "proceed_to_independent_verification",
            "reason": "all_required_coaches_approved",
            "parsed_json": True,
            "blockers": [],
            "evidence_refs": deduped_strings(
                [ref for decision in valid for ref in normalized_string_list(decision.get("evidence_refs"))]
            ),
            "verification_results": deduped_strings(
                [item for decision in valid for item in normalized_string_list(decision.get("verification_results"))]
            ),
            "impact_analysis": merge_impact_analyses(valid),
            "answer": coach_feedback_summary(valid),
            "merge_ready_effective": "yes",
            "raw_merge_ready": "",
            "raw_rework_required": "",
            "selected_subagents": selected_subagents,
            "valid_result_count": valid_result_count,
            "required_result_count": normalized_required,
            "merge_policy": merge_policy,
            "coach_results": decisions,
        }

    blocking_decisions = rework_decisions or valid
    blockers = deduped_strings(
        [item for decision in blocking_decisions for item in normalized_string_list(decision.get("blockers"))]
    )
    feedback = coach_feedback_summary(blocking_decisions)
    recommended_next_actions = deduped_strings(
        [str(decision.get("recommended_next_action", "")).strip() for decision in blocking_decisions]
    )
    reason = "; ".join(blockers) or feedback or "coach_rework_required"
    return {
        "approved": False,
        "coach_decision": "return_for_rework",
        "payload_state": "return_for_rework",
        "invalid_reasons": [],
        "rework_required": "yes",
        "coach_feedback": feedback,
        "recommended_next_action": "; ".join(recommended_next_actions),
        "reason": reason,
        "parsed_json": True,
        "blockers": blockers,
        "evidence_refs": deduped_strings(
            [ref for decision in blocking_decisions for ref in normalized_string_list(decision.get("evidence_refs"))]
        ),
        "verification_results": deduped_strings(
            [item for decision in blocking_decisions for item in normalized_string_list(decision.get("verification_results"))]
        ),
        "impact_analysis": merge_impact_analyses(blocking_decisions),
        "answer": feedback,
        "merge_ready_effective": "no",
        "raw_merge_ready": "",
        "raw_rework_required": "",
        "selected_subagents": selected_subagents,
        "valid_result_count": valid_result_count,
        "required_result_count": normalized_required,
        "merge_policy": merge_policy,
        "coach_results": decisions,
    }


def coach_decision_from_manifest(manifest: dict[str, Any]) -> dict[str, Any]:
    results = manifest.get("results", [])
    if not isinstance(results, list):
        results = []
    required_results = int(manifest.get("required_result_count", 1) or 1)
    merge_policy = str(manifest.get("merge_policy", "unanimous_approve_rework_bias")).strip() or "unanimous_approve_rework_bias"
    decisions = [coach_decision_from_result(item) for item in results if isinstance(item, dict)]
    if decisions:
        return merge_coach_decisions(decisions, required_results=required_results, merge_policy=merge_policy)
    return with_coach_feedback_provenance(
        {
        "approved": bool(manifest.get("synthesis_ready", False)),
        "coach_decision": "approved" if manifest.get("synthesis_ready", False) else "return_for_rework",
        "rework_required": "no" if manifest.get("synthesis_ready", False) else "yes",
        "coach_feedback": "",
        "recommended_next_action": "",
        "reason": str(manifest.get("status", "coach_review_incomplete")),
        "parsed_json": False,
        "output_file": "",
        "subagent": "",
        },
        primary_source="manifest_summary",
        source_chain=["manifest_summary"],
    )


def write_rework_handoff(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
    *,
    original_prompt: str,
    coach_decision: dict[str, Any],
    attempt_count: int,
    max_coach_passes: int,
) -> dict[str, Any]:
    coach_decision = with_coach_feedback_provenance(coach_decision)
    path = rework_handoff_path(task_id, task_class)
    path.parent.mkdir(parents=True, exist_ok=True)
    fresh_prompt_text = fresh_rework_prompt_text(
        original_prompt,
        task_class,
        coach_decision,
        attempt_count=attempt_count,
        max_coach_passes=max_coach_passes,
    )
    payload = {
        "ts": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "status": "writer_rework_ready",
        "attempt_count": attempt_count,
        "max_coach_passes": max_coach_passes,
        "fresh_start_required": True,
        "route_receipt_hash": route_receipt_hash(route),
        "route_receipt": route_receipt_payload(route),
        "original_prompt_sha256": digest_text(original_prompt),
        "original_prompt_text": original_prompt,
        "fresh_prompt_sha256": digest_text(fresh_prompt_text),
        "fresh_prompt_text": fresh_prompt_text,
        "coach_delta": {
            "coach_feedback": str(coach_decision.get("coach_feedback", "")).strip(),
            "reason": str(coach_decision.get("reason", "")).strip(),
            "recommended_next_action": str(coach_decision.get("recommended_next_action", "")).strip(),
            "blockers": normalized_string_list(coach_decision.get("blockers")),
            "evidence_refs": normalized_string_list(coach_decision.get("evidence_refs")),
            "verification_results": normalized_string_list(coach_decision.get("verification_results")),
            "impact_analysis": normalized_impact_analysis(coach_decision.get("impact_analysis")),
            "subagent": str(coach_decision.get("subagent", "")).strip(),
            "subagents": normalized_string_list(coach_decision.get("selected_subagents")),
            "feedback_source": str(coach_decision.get("feedback_source", "")).strip(),
            "feedback_sources": normalized_string_list(coach_decision.get("feedback_sources")),
            "coach_results": coach_decision.get("coach_results", []),
            "output_file": str(coach_decision.get("output_file", "")).strip(),
        },
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return {
        "path": str(path),
        "status": payload["status"],
        "fresh_prompt_sha256": payload["fresh_prompt_sha256"],
    }


def effective_writer_prompt(
    task_id: str,
    task_class: str,
    route: dict[str, Any],
    prompt_file: Path,
    output_dir: Path,
) -> tuple[Path, dict[str, Any]]:
    handoff_ok, handoff, handoff_error = validate_rework_handoff(task_id, task_class, route)
    if not handoff_ok:
        metadata = {
            "mode": "original_prompt",
            "input_prompt_file": str(prompt_file),
            "effective_prompt_file": str(prompt_file),
            "rework_handoff_path": str(rework_handoff_path(task_id, task_class)),
            "rework_handoff_status": handoff_error or "not_present",
        }
        if handoff:
            metadata["rework_handoff_payload_status"] = str(handoff.get("status", "")).strip()
        return prompt_file, metadata

    effective_path = output_dir / "writer.rework.prompt.txt"
    effective_path.write_text(str(handoff.get("fresh_prompt_text", "")).strip() + "\n", encoding="utf-8")
    metadata = {
        "mode": "fresh_rework_handoff",
        "input_prompt_file": str(prompt_file),
        "effective_prompt_file": str(effective_path),
        "rework_handoff_path": str(rework_handoff_path(task_id, task_class)),
        "rework_handoff_status": "ready",
        "rework_attempt_count": int(handoff.get("attempt_count", 0) or 0),
        "fresh_prompt_sha256": str(handoff.get("fresh_prompt_sha256", "")).strip(),
    }
    return effective_path, metadata


def run_coach_ensemble(
    *,
    task_id: str,
    writer_task_class: str,
    coach_task_class: str,
    prompt_file: Path,
    output_dir: Path,
    workdir: Path,
    snapshot: dict[str, Any],
    route: dict[str, Any],
    coach_plan: dict[str, Any],
) -> tuple[Path, dict[str, Any], dict[str, Any]]:
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = output_dir / "manifest.json"
    selected_subagents = coach_selected_subagents(coach_plan)
    fallback_subagents = deduped_strings(
        [
            str(item.get("subagent", "")).strip()
            for item in coach_plan.get("fallback_subagents", [])
            if isinstance(item, dict)
        ]
    )
    requested_subagents = deduped_strings([*selected_subagents, *fallback_subagents])
    required_results = max(1, int(coach_plan.get("min_results", 0) or len(selected_subagents) or 1))
    merge_policy = policy_value(coach_plan.get("merge_policy"), "unanimous_approve_rework_bias")
    coach_dispatch_route = subagent_system.route_subagent(coach_task_class)
    coach_dispatch_route = {
        **coach_dispatch_route,
        "selected_subagent": selected_subagents[0] if selected_subagents else coach_dispatch_route.get("selected_subagent"),
        "fallback_subagents": [
            {"subagent": subagent}
            for subagent in deduped_strings(requested_subagents[1:])
        ],
    }
    results: list[dict[str, Any]] = []
    decisions: list[dict[str, Any]] = []
    attempted_subagents: list[str] = []
    fallback_used = False

    for subagent_name in requested_subagents:
        if (
            subagent_name == "internal_subagents"
            and policy_value(
                ((coach_dispatch_route.get("dispatch_policy") or {}).get("internal_route_authorized")),
                "no",
            ) != "yes"
        ):
            continue
        if len([decision for decision in decisions if coach_decision_is_valid(decision)]) >= required_results and subagent_name in fallback_subagents:
            break
        subagent_cfg = candidate_subagent_cfg(snapshot, subagent_name)
        dispatch_mode = "fallback" if subagent_name in fallback_subagents else "fanout"
        result = run_subagent(
            task_id,
            coach_task_class,
            subagent_name,
            prompt_file,
            output_dir / f"{subagent_name}.txt",
            workdir,
            coach_dispatch_route,
            subagent_cfg,
            dispatch_mode,
        )
        decision = coach_decision_from_result(result)
        results.append(result)
        decisions.append(decision)
        attempted_subagents.append(subagent_name)
        if subagent_name in fallback_subagents:
            fallback_used = True

    merged_decision = merge_coach_decisions(
        decisions,
        required_results=required_results,
        merge_policy=merge_policy,
    )
    status = "coach_approved" if merged_decision.get("approved") else str(merged_decision.get("coach_decision", "coach_failed"))
    manifest = {
        "generated_at": now_utc(),
        "task_id": task_id,
        "writer_task_class": writer_task_class,
        "coach_task_class": coach_task_class,
        "requested_subagents": requested_subagents,
        "selected_subagents": selected_subagents,
        "fallback_subagents": fallback_subagents,
        "attempted_subagents": attempted_subagents,
        "fallback_used": fallback_used,
        "required_result_count": required_results,
        "valid_result_count": int(merged_decision.get("valid_result_count", 0) or 0),
        "merge_policy": merge_policy,
        "decision_ready": merged_decision.get("parsed_json") is True,
        "synthesis_ready": merged_decision.get("approved") is True,
        "subagent_exhausted": int(merged_decision.get("valid_result_count", 0) or 0) < required_results,
        "result_count": len(results),
        "status": status,
        "phase": "completed" if merged_decision.get("approved") else "blocked",
        "results": results,
        "coach_results": decisions,
        "decision_summary": merged_decision,
        "manifest_path": str(manifest_path),
    }
    write_manifest(manifest_path, manifest)
    return manifest_path, manifest, merged_decision


def run_verification_phase(
    *,
    task_id: str,
    task_class: str,
    prompt_file: Path,
    output_dir: Path,
    workdir: Path,
    route: dict[str, Any],
    merge_summary: dict[str, Any],
    post_arbitration_merge_summary: dict[str, Any],
    results: list[dict[str, Any]],
) -> tuple[dict[str, Any], bool]:
    verification_plan = route.get("verification_plan") or {}
    route_budget = route_budget_policy(route)
    if route.get("independent_verification_required") != "yes":
        return {"required": False, "status": "not_required"}, True
    if int(route_budget.get("max_verification_passes", 0) or 0) <= 0:
        return {
            "required": True,
            "status": "blocked",
            "reason": "verification_pass_cap_exceeded",
            "verification_plan": verification_plan,
        }, False
    verification_task_class = str(verification_plan.get("route_task_class", "")).strip()
    selected_subagent = verification_plan.get("selected_subagent")
    if not verification_task_class or not selected_subagent:
        return {
            "required": True,
            "status": "blocked",
            "reason": "missing_verification_plan",
            "verification_plan": verification_plan,
        }, False

    verification_prompt_file = output_dir / "verification.prompt.txt"
    verification_output_dir = output_dir / "verification"
    verification_output_dir.mkdir(parents=True, exist_ok=True)
    verification_prompt_file.write_text(
        verification_prompt_text(
            original_prompt=read_prompt(prompt_file),
            task_class=task_class,
            verification_task_class=verification_task_class,
            merge_summary=merge_summary,
            post_arbitration_merge_summary=post_arbitration_merge_summary,
            results=results,
        ),
        encoding="utf-8",
    )

    command = [
        sys.executable,
        str(Path(__file__).resolve()),
        "ensemble",
        task_id,
        verification_task_class,
        str(verification_prompt_file),
        str(verification_output_dir),
        str(workdir),
    ]
    completed = subprocess.run(
        command,
        cwd=str(workdir),
        capture_output=True,
        text=True,
        check=False,
    )
    manifest_path = Path((completed.stdout or "").strip().splitlines()[-1]) if (completed.stdout or "").strip() else None
    verification_manifest: dict[str, Any] = {}
    if manifest_path and manifest_path.exists():
        try:
            verification_manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            verification_manifest = {}
    synthesis_ready = completed.returncode == 0 and verification_manifest.get("synthesis_ready", False) is True
    status = "completed" if synthesis_ready else "blocked"
    if completed.returncode == 3:
        status = "verification_pending"
    return {
        "required": True,
        "status": status,
        "task_class": verification_task_class,
        "selected_subagent": selected_subagent,
        "command": command,
        "return_code": completed.returncode,
        "stdout": (completed.stdout or "").strip(),
        "stderr": (completed.stderr or "").strip(),
        "manifest_path": str(manifest_path) if manifest_path else "",
        "manifest": verification_manifest,
    }, synthesis_ready


def parse_json_object(text: str) -> dict[str, Any]:
    payload = worker_packet_gate.extract_json_payload(text)
    if payload:
        return payload
    stripped = text.strip()
    candidates: list[str] = []
    fenced = re.findall(r"```json\s*(\{.*?\})\s*```", text, flags=re.DOTALL)
    candidates.extend(reversed(fenced))
    inline_objects = re.findall(r"(\{[^{}]*\})", text, flags=re.DOTALL)
    candidates.extend(reversed(inline_objects))
    if stripped.startswith("{") and stripped.endswith("}"):
        candidates.append(stripped)
    for candidate in candidates:
        if not candidate:
            continue
        try:
            parsed = json.loads(candidate)
        except json.JSONDecodeError:
            continue
        if isinstance(parsed, dict):
            return parsed
    return {}


def parse_arbitration_decision(text: str, allowed_cluster_ids: list[str]) -> dict[str, Any]:
    allowed = {cluster_id for cluster_id in allowed_cluster_ids if cluster_id}
    payload = parse_json_object(text)
    decision = str(payload.get("decision", "")).strip().casefold()
    selected_cluster_id = str(payload.get("selected_cluster_id", "")).strip()
    if not selected_cluster_id:
        match = re.search(r"(?:selected_cluster_id|cluster_id)[^A-Za-z0-9]+([a-f0-9]{6,12})", text, flags=re.IGNORECASE)
        if match:
            selected_cluster_id = match.group(1)
    confidence = str(payload.get("confidence", "medium")).strip().casefold() or "medium"
    rationale = str(payload.get("rationale", "")).strip()

    if decision not in {"select_cluster", "no_decision"}:
        decision = "select_cluster" if selected_cluster_id in allowed else "no_decision"
    if selected_cluster_id not in allowed:
        selected_cluster_id = ""
        if decision == "select_cluster":
            decision = "no_decision"

    return {
        "decision": decision,
        "selected_cluster_id": selected_cluster_id,
        "confidence": confidence if confidence in {"low", "medium", "high"} else "medium",
        "rationale": rationale,
        "parsed_json": bool(payload),
    }


def validate_arbitration_output_text(text: str, allowed_cluster_ids: list[str]) -> list[str]:
    payload = parse_json_object(text)
    if not payload:
        return ["arbitration output must be valid JSON"]
    errors: list[str] = []
    decision = str(payload.get("decision", "")).strip().casefold()
    selected_cluster_id = str(payload.get("selected_cluster_id", "")).strip()
    confidence = str(payload.get("confidence", "")).strip().casefold()
    rationale = payload.get("rationale")
    allowed = {cluster_id for cluster_id in allowed_cluster_ids if cluster_id}

    if decision not in {"select_cluster", "no_decision"}:
        errors.append("arbitration decision must be select_cluster or no_decision")
    if decision == "select_cluster" and selected_cluster_id not in allowed:
        errors.append("arbitration selected_cluster_id must be one of the allowed cluster ids")
    if decision == "no_decision" and selected_cluster_id:
        errors.append("arbitration selected_cluster_id must be empty when decision is no_decision")
    if confidence not in {"low", "medium", "high"}:
        errors.append("arbitration confidence must be low, medium, or high")
    if not isinstance(rationale, str) or not rationale.strip():
        errors.append("arbitration rationale must be a non-empty string")
    return errors


def apply_arbitration_decision(
    merge_summary: dict[str, Any],
    arbitration_decision: dict[str, Any],
    arbitration_subagent: str,
) -> dict[str, Any]:
    post_summary = clone_json_payload(merge_summary)
    post_summary["arbitrated_consensus"] = False
    post_summary["arbitration_subagent"] = arbitration_subagent
    post_summary["arbitration_decision"] = arbitration_decision.get("decision", "no_decision")
    post_summary["arbitration_selected_cluster_id"] = arbitration_decision.get("selected_cluster_id", "")
    post_summary["arbitration_confidence"] = arbitration_decision.get("confidence", "medium")
    post_summary["arbitration_rationale"] = arbitration_decision.get("rationale", "")

    selected_cluster_id = str(arbitration_decision.get("selected_cluster_id", ""))
    if arbitration_decision.get("decision") == "select_cluster" and selected_cluster_id:
        open_conflicts = post_summary.get("open_conflicts", [])
        selected_cluster = next(
            (cluster for cluster in open_conflicts if cluster.get("cluster_id") == selected_cluster_id),
            None,
        )
        if selected_cluster is not None:
            post_summary["dominant_finding"] = selected_cluster
            post_summary["open_conflicts"] = [
                cluster for cluster in open_conflicts if cluster.get("cluster_id") != selected_cluster_id
            ]
            post_summary["consensus_mode"] = "arbitrated"
            post_summary["tie_break_recommended"] = False
            post_summary["tie_break_reason"] = ""
            post_summary["orchestrator_review_required"] = False
            post_summary["arbitrated_consensus"] = True
            return post_summary

    post_summary["consensus_mode"] = "inconclusive"
    post_summary["tie_break_recommended"] = True
    post_summary["tie_break_reason"] = "arbitration_inconclusive"
    post_summary["orchestrator_review_required"] = True
    return post_summary


def unresolved_arbitration_summary(merge_summary: dict[str, Any], tie_break_reason: str) -> dict[str, Any]:
    post_summary = clone_json_payload(merge_summary)
    post_summary["consensus_mode"] = "unresolved"
    post_summary["tie_break_recommended"] = True
    post_summary["tie_break_reason"] = tie_break_reason
    post_summary["orchestrator_review_required"] = True
    post_summary["arbitrated_consensus"] = False
    return post_summary


def run_bounded_arbitration(
    task_id: str,
    task_class: str,
    prompt_file: Path,
    output_dir: Path,
    workdir: Path,
    snapshot: dict[str, Any],
    route: dict[str, Any],
    requested_fanout: list[str],
    results: list[dict[str, Any]],
    merge_summary: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, Any]]:
    arbitration: dict[str, Any] = {
        "requested": merge_summary.get("tie_break_reason") == "semantic_conflict_without_majority",
        "trigger_reason": merge_summary.get("tie_break_reason", ""),
        "status": "skipped",
        "selected_subagent": None,
        "selection_reason": "",
        "subagent_reused": False,
        "candidate_subagents": [],
        "decision": {
            "decision": "no_decision",
            "selected_cluster_id": "",
            "confidence": "medium",
            "rationale": "",
            "parsed_json": False,
        },
    }
    if not arbitration["requested"]:
        return arbitration, merge_summary

    candidates = arbitration_candidates(route, snapshot, requested_fanout, results)
    arbitration["candidate_subagents"] = [item["subagent"] for item in candidates]
    if not candidates:
        arbitration["status"] = "unavailable"
        arbitration["selection_reason"] = "no_supported_arbitration_subagent"
        return arbitration, unresolved_arbitration_summary(merge_summary, "arbitration_subagent_unavailable")

    selected = candidates[0]
    subagent_name = str(selected["subagent"])
    arbitration["selected_subagent"] = subagent_name
    arbitration["selection_reason"] = str(selected["selection_reason"])
    arbitration["subagent_reused"] = bool(selected["used"])

    arbitration_prompt_file = output_dir / "arbitration-prompt.txt"
    arbitration_output_file = output_dir / f"{subagent_name}.arbitration.txt"
    arbitration_prompt_file.write_text(
        arbitration_prompt_text(read_prompt(prompt_file), task_class, merge_summary, results),
        encoding="utf-8",
    )
    arbitration["prompt_file"] = str(arbitration_prompt_file)
    arbitration["output_file"] = str(arbitration_output_file)

    payload = run_subagent(
        task_id,
        task_class,
        subagent_name,
        arbitration_prompt_file,
        arbitration_output_file,
        workdir,
        route,
        candidate_subagent_cfg(snapshot, subagent_name),
        "arbitration",
    )
    arbitration["run"] = payload
    arbitration["status"] = str(payload.get("status", "failure"))

    if payload.get("status") != "success":
        return arbitration, unresolved_arbitration_summary(merge_summary, "arbitration_subagent_failed")

    allowed_cluster_ids = [str(cluster.get("cluster_id", "")) for cluster in merge_summary.get("open_conflicts", [])]
    arbitration_output_errors = validate_arbitration_output_text(load_output_text(arbitration_output_file), allowed_cluster_ids)
    if arbitration_output_errors:
        arbitration["status"] = "output_contract_invalid"
        arbitration["output_contract_errors"] = arbitration_output_errors
        return arbitration, unresolved_arbitration_summary(merge_summary, "arbitration_output_contract_invalid")
    arbitration["decision"] = parse_arbitration_decision(load_output_text(arbitration_output_file), allowed_cluster_ids)
    return arbitration, apply_arbitration_decision(merge_summary, arbitration["decision"], subagent_name)


def run_ensemble(argv: list[str]) -> int:
    if len(argv) < 6:
        print(
            "Usage: python3 _vida/scripts/subagent-dispatch.py ensemble <task_id> <task_class> <prompt_file> <output_dir> [workdir]",
            file=sys.stderr,
        )
        return 1
    task_id, task_class, prompt_file_raw, output_dir_raw = argv[2:6]
    workdir = Path(argv[6]).resolve() if len(argv) > 6 else ROOT_DIR
    prompt_file = Path(prompt_file_raw).resolve()
    output_dir = Path(output_dir_raw).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    snapshot, route = route_snapshot(task_class, task_id)
    route_budget = route_budget_policy(route)
    budget_blockers = route_budget_blockers(route)
    requested_fanout = ensemble_subagents(route)
    max_parallel_agents = max(1, int(snapshot.get("agent_system", {}).get("max_parallel_agents", 1)))
    primary_fanout = requested_fanout[:max_parallel_agents]
    min_results = int(route.get("fanout_min_results", 0))
    required_results = max(1, min_results)
    manifest_path = output_dir / "manifest.json"
    if budget_blockers:
        manifest = {
            "generated_at": now_utc(),
            "task_id": task_id,
            "task_class": task_class,
            "workdir": str(workdir),
            "route": route,
            "route_receipt": route_receipt_payload(route),
            "status": "blocked",
            "phase": "budget_blocked",
            "review_state": "review_failed",
            "target_review_state": subagent_system.target_review_state_for(route_risk_class(route)),
            "target_manifest_review_state": subagent_system.target_manifest_review_state_for(route_risk_class(route)),
            "budget_blockers": budget_blockers,
            "results": [],
        }
        write_manifest(manifest_path, manifest)
        print(str(manifest_path))
        return 2
    run_holder = f"ensemble:{task_id}:{task_class}:{uuid.uuid4().hex[:8]}"
    lease_result = subagent_system.acquire_lease("ensemble", f"{task_id}:{task_class}", run_holder, ttl_seconds=3600)
    if lease_result.get("status") != "acquired":
        manifest = {
            "generated_at": now_utc(),
            "task_id": task_id,
            "task_class": task_class,
            "workdir": str(workdir),
            "route": route,
            "status": "blocked",
            "phase": "lease_blocked",
            "review_state": "review_failed",
            "target_review_state": subagent_system.target_review_state_for(route_risk_class(route)),
            "target_manifest_review_state": subagent_system.target_manifest_review_state_for(route_risk_class(route)),
            "lease": lease_result,
            "results": [],
        }
        write_manifest(manifest_path, manifest)
        print(str(manifest_path))
        return 2

    results: list[dict[str, Any]] = []
    manifest: dict[str, Any] = {
        "generated_at": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "workdir": str(workdir),
        "route": route,
        "route_receipt": route_receipt_payload(route),
        "requested_fanout_subagents": requested_fanout,
        "fanout_subagents": primary_fanout,
        "fanout_min_results": min_results,
        "max_parallel_agents": max_parallel_agents,
        "risk_class": route_risk_class(route),
        "review_state": "review_pending",
        "target_review_state": subagent_system.target_review_state_for(route_risk_class(route)),
        "target_manifest_review_state": subagent_system.target_manifest_review_state_for(route_risk_class(route)),
        "success_count": 0,
        "useful_progress_count": 0,
        "subagent_exhausted": False,
        "fallback_used": False,
        "merge_summary": {},
        "arbitration": {},
        "post_arbitration_merge_summary": {},
        "results": [],
        "lease": lease_result.get("lease", {}),
        "lease_renew_count": 0,
        "active_subagents": [],
        "active_count": 0,
        "status": "running",
        "phase": "fanout_running",
    }
    write_manifest(manifest_path, manifest)
    ensemble_started = time.monotonic()
    max_total_runtime_seconds = int(route_budget.get("max_total_runtime_seconds", 0) or 0)
    runtime_budget_blocked = False
    launches: dict[str, dict[str, Any]] = {}
    for subagent_name in primary_fanout:
        launch = start_subagent_process(
            task_id,
            task_class,
            subagent_name,
            prompt_file,
            output_dir / f"{subagent_name}.txt",
            workdir,
            route,
            candidate_subagent_cfg(snapshot, subagent_name),
            "fanout",
        )
        if "result" in launch:
            results.append(launch["result"])
        else:
            launches[subagent_name] = launch

    manifest["results"] = sorted(results, key=lambda item: item["subagent"])
    manifest["success_count"] = sum(
        1 for item in results if item.get("status") == "success" and item.get("merge_ready") is True
    )
    manifest["active_subagents"] = manifest_active_subagents(launches)
    manifest["active_count"] = len(launches)
    write_manifest(manifest_path, manifest)

    while launches:
        completed_subagents: list[str] = []
        loop_progress = False
        if max_total_runtime_seconds > 0 and (time.monotonic() - ensemble_started) > max_total_runtime_seconds:
            for subagent_name, launch in list(launches.items()):
                results.append(
                    terminate_subagent_process(
                        launch,
                        f"route exceeded max_total_runtime_seconds ({max_total_runtime_seconds}s)",
                        status_override="timeout",
                        exit_code_override=124,
                    )
                )
                completed_subagents.append(subagent_name)
            runtime_budget_blocked = True
        lease_payload = manifest.get("lease", {})
        lease_acquired_at = subagent_system.parse_utc_timestamp(lease_payload.get("acquired_at")) if isinstance(lease_payload, dict) else None
        lease_ttl_seconds = 3600
        if lease_acquired_at is not None:
            lease_expires_at = subagent_system.parse_utc_timestamp(lease_payload.get("expires_at"))
            if lease_expires_at is not None:
                lease_ttl_seconds = max(60, int((lease_expires_at - lease_acquired_at).total_seconds()))
        last_renewed_at = subagent_system.parse_utc_timestamp(lease_payload.get("renewed_at")) if isinstance(lease_payload, dict) else None
        renew_base = last_renewed_at or lease_acquired_at
        if renew_base is not None and (subagent_system.now_utc_dt() - renew_base).total_seconds() >= max(60, lease_ttl_seconds // 2):
            renew_result = subagent_system.renew_lease("ensemble", f"{task_id}:{task_class}", run_holder, lease_ttl_seconds)
            if renew_result.get("status") == "renewed":
                manifest["lease"] = renew_result.get("lease", manifest.get("lease", {}))
                manifest["lease_renew_count"] = int(manifest.get("lease_renew_count", 0) or 0) + 1
                write_manifest(manifest_path, manifest)
        for subagent_name, launch in list(launches.items()):
            process: subprocess.Popen[str] = launch["process"]
            progress = launch_progress_snapshot(launch)
            elapsed = time.monotonic() - float(launch["started"])
            effective_runtime_seconds = int(launch.get("effective_runtime_seconds", launch["max_runtime_seconds"]))
            if (
                process.poll() is None
                and not progress.get("observed_output")
                and elapsed > int(launch.get("startup_timeout_seconds", 45))
            ):
                results.append(
                    terminate_subagent_process(
                        launch,
                        f"cli subagent hit startup timeout without output ({launch.get('startup_timeout_seconds', 45)}s)",
                        status_override="timeout",
                        exit_code_override=124,
                    )
                )
                completed_subagents.append(subagent_name)
                loop_progress = True
                continue
            if (
                process.poll() is None
                and not progress.get("useful_progress")
                and int(progress.get("idle_seconds", 0)) > int(launch.get("no_output_timeout_seconds", 120))
            ):
                results.append(
                    terminate_subagent_process(
                        launch,
                        f"cli subagent hit no-output timeout without useful progress ({launch.get('no_output_timeout_seconds', 120)}s)",
                        status_override="timeout",
                        exit_code_override=124,
                    )
                )
                completed_subagents.append(subagent_name)
                loop_progress = True
                continue
            if (
                process.poll() is None
                and elapsed > effective_runtime_seconds
                and launch.get("runtime_extension_applied") is not True
                and progress.get("useful_progress")
                and int(progress.get("idle_seconds", 0)) <= 45
            ):
                extension = runtime_extension_seconds(launch)
                launch["runtime_extension_applied"] = True
                launch["effective_runtime_seconds"] = effective_runtime_seconds + extension
                loop_progress = True
                continue
            if process.poll() is None and elapsed > int(launch.get("effective_runtime_seconds", launch["max_runtime_seconds"])):
                results.append(
                    terminate_subagent_process(
                        launch,
                        f"cli subagent exceeded runtime limit ({launch.get('effective_runtime_seconds', launch['max_runtime_seconds'])}s)",
                        status_override="timeout",
                        exit_code_override=124,
                    )
                )
                completed_subagents.append(subagent_name)
                loop_progress = True
                continue
            if (
                process.poll() is None
                and progress.get("useful_progress")
                and int(progress.get("idle_seconds", 0)) > int(launch.get("progress_idle_timeout_seconds", 90))
            ):
                results.append(
                    terminate_subagent_process(
                        launch,
                        f"cli subagent stalled after useful progress ({launch.get('progress_idle_timeout_seconds', 90)}s idle)",
                        status_override="timeout",
                        exit_code_override=124,
                    )
                )
                completed_subagents.append(subagent_name)
                loop_progress = True
                continue
            if process.poll() is not None:
                results.append(finalize_subagent_process(launch))
                completed_subagents.append(subagent_name)
                loop_progress = True

        for subagent_name in completed_subagents:
            launches.pop(subagent_name, None)

        success_count = sum(
            1 for item in results if item.get("status") == "success" and item.get("merge_ready") is True
        )
        manifest["results"] = sorted(results, key=lambda item: item["subagent"])
        manifest["success_count"] = success_count
        manifest["useful_progress_count"] = sum(1 for launch in launches.values() if launch.get("useful_progress"))
        manifest["active_subagents"] = manifest_active_subagents(launches)
        manifest["active_count"] = len(launches)
        write_manifest(manifest_path, manifest)

        if success_count >= required_results and launches:
            for subagent_name, launch in list(launches.items()):
                results.append(
                    terminate_subagent_process(
                        launch,
                        "terminated after required merge-ready ensemble results were reached",
                    )
                )
                launches.pop(subagent_name, None)
            manifest["results"] = sorted(results, key=lambda item: item["subagent"])
            manifest["success_count"] = success_count
            manifest["useful_progress_count"] = 0
            manifest["active_subagents"] = []
            manifest["active_count"] = 0
            write_manifest(manifest_path, manifest)
            break

        max_possible_successes = success_count + len(launches)
        if max_possible_successes < required_results and launches:
            for subagent_name, launch in list(launches.items()):
                results.append(
                    terminate_subagent_process(
                        launch,
                        "terminated because fanout_min_results became unreachable with remaining subagents",
                    )
                )
                launches.pop(subagent_name, None)
            manifest["results"] = sorted(results, key=lambda item: item["subagent"])
            manifest["success_count"] = success_count
            manifest["useful_progress_count"] = 0
            manifest["subagent_exhausted"] = True
            manifest["active_subagents"] = []
            manifest["active_count"] = 0
            write_manifest(manifest_path, manifest)
            break

        if not loop_progress and launches:
            time.sleep(0.5)
        if runtime_budget_blocked:
            break

    success_count = sum(
        1 for item in results if item.get("status") == "success" and item.get("merge_ready") is True
    )
    if runtime_budget_blocked:
        manifest.update(
            {
                "generated_at": now_utc(),
                "results": sorted(results, key=lambda item: item["subagent"]),
                "success_count": success_count,
                "useful_progress_count": sum(1 for item in results if item.get("useful_progress")),
                "active_subagents": [],
                "active_count": 0,
                "status": "blocked",
                "phase": "budget_blocked",
                "budget_blockers": ["max_total_runtime_exceeded"],
            }
        )
        release_result = subagent_system.release_lease("ensemble", f"{task_id}:{task_class}", run_holder)
        manifest["lease"] = {
            **manifest.get("lease", {}),
            "release_status": release_result.get("status"),
        }
        write_manifest(manifest_path, manifest)
        print(str(manifest_path))
        return 2
    fallback_used = False
    if success_count < min_results:
        manifest["phase"] = "fallback_running"
        manifest["subagent_exhausted"] = False
        manifest["fallback_used"] = False
        manifest["useful_progress_count"] = sum(1 for item in results if item.get("useful_progress"))
        manifest["active_subagents"] = []
        manifest["active_count"] = 0
        write_manifest(manifest_path, manifest)
        for item in route.get("fallback_subagents", []):
            subagent_name = item.get("subagent")
            if not subagent_name or subagent_name in primary_fanout:
                continue
            subagent_cfg = candidate_subagent_cfg(snapshot, subagent_name)
            if not subagent_supports_dispatch(subagent_cfg):
                continue
            fallback_used = True
            manifest["fallback_used"] = True
            manifest["active_subagents"] = []
            manifest["active_count"] = 0
            write_manifest(manifest_path, manifest)
            result = run_subagent(
                task_id,
                task_class,
                subagent_name,
                prompt_file,
                output_dir / f"{subagent_name}.txt",
                workdir,
                route,
                subagent_cfg,
                "fallback",
            )
            results.append(result)
            manifest["results"] = sorted(results, key=lambda item: item["subagent"])
            if result.get("status") == "success" and result.get("merge_ready") is True:
                success_count += 1
            manifest["success_count"] = success_count
            manifest["useful_progress_count"] = sum(1 for item in results if item.get("useful_progress"))
            manifest["active_subagents"] = []
            manifest["active_count"] = 0
            write_manifest(manifest_path, manifest)
            if success_count >= min_results:
                break

    manifest["phase"] = "merge_evaluating"
    write_manifest(manifest_path, manifest)
    merge_summary = build_merge_summary(
        results,
        str(route.get("merge_policy", "single_subagent")),
        min_results,
        subagent_scores=route_subagent_scores(route),
    )
    manifest["phase"] = "arbitration_running" if merge_summary.get("tie_break_recommended") else "finalizing"
    write_manifest(manifest_path, manifest)
    arbitration, post_arbitration_merge_summary = run_bounded_arbitration(
        task_id,
        task_class,
        prompt_file,
        output_dir,
        workdir,
        snapshot,
        route,
        requested_fanout,
        results,
        merge_summary,
    )
    manifest = {
        **manifest,
        "generated_at": now_utc(),
        "success_count": success_count,
        "useful_progress_count": sum(1 for item in results if item.get("useful_progress")),
        "subagent_exhausted": success_count < required_results,
        "fallback_used": fallback_used,
        "merge_summary": merge_summary,
        "arbitration": arbitration,
        "post_arbitration_merge_summary": post_arbitration_merge_summary,
        "verification": {},
        "results": sorted(results, key=lambda item: item["subagent"]),
        "active_subagents": [],
        "active_count": 0,
        "verification_pending": False,
        "synthesis_ready": False,
        "review_state": manifest_review_state(
            post_arbitration_merge_summary or merge_summary,
            route_risk_class(route),
        ),
        "target_review_state": subagent_system.target_review_state_for(route_risk_class(route)),
        "target_manifest_review_state": subagent_system.target_manifest_review_state_for(route_risk_class(route)),
        "status": "subagent_exhausted",
        "phase": "subagent_exhausted",
    }
    decision_ready = success_count >= required_results or (post_arbitration_merge_summary or merge_summary).get("decision_ready")
    synthesis_ready = decision_ready
    if decision_ready:
        verification_result, verification_synthesis_ready = run_verification_phase(
            task_id=task_id,
            task_class=task_class,
            prompt_file=prompt_file,
            output_dir=output_dir,
            workdir=workdir,
            route=route,
            merge_summary=merge_summary,
            post_arbitration_merge_summary=post_arbitration_merge_summary,
            results=results,
        )
        manifest["verification"] = verification_result
        if verification_result.get("required"):
            manifest["verification_pending"] = not verification_synthesis_ready
            synthesis_ready = verification_synthesis_ready
    else:
        manifest["verification"] = {"required": route.get("independent_verification_required") == "yes", "status": "skipped_primary_not_ready"}
        manifest["verification_pending"] = route.get("independent_verification_required") == "yes"
    manifest["synthesis_ready"] = synthesis_ready
    if manifest["verification_pending"]:
        manifest["review_state"] = "review_pending"
    if synthesis_ready:
        manifest["status"] = "completed"
        manifest["phase"] = "completed"
    elif decision_ready and manifest["verification_pending"]:
        manifest["status"] = "verification_pending"
        manifest["phase"] = "verification_pending"
    release_result = subagent_system.release_lease("ensemble", f"{task_id}:{task_class}", run_holder)
    manifest["lease"] = {
        **manifest.get("lease", {}),
        "release_status": release_result.get("status"),
    }
    write_manifest(manifest_path, manifest)
    print(str(manifest_path))
    if synthesis_ready:
        return 0
    if manifest["verification_pending"]:
        return 3
    return 2


def run_prepare_execution(argv: list[str]) -> int:
    if len(argv) < 6:
        print(
            "Usage: python3 _vida/scripts/subagent-dispatch.py prepare-execution <task_id> <writer_task_class> <prompt_file> <output_dir> [workdir]",
            file=sys.stderr,
        )
        return 1
    task_id, writer_task_class, prompt_file_raw, output_dir_raw = argv[2:6]
    workdir = Path(argv[6]).resolve() if len(argv) > 6 else ROOT_DIR
    prompt_file = Path(prompt_file_raw).resolve()
    output_dir = Path(output_dir_raw).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    snapshot, writer_route = route_snapshot(writer_task_class, task_id)
    manifest_path = output_dir / "prepare-execution.json"
    analysis_plan = writer_route.get("analysis_plan") or {}
    writer_route_receipt_path = write_route_receipt(task_id, writer_task_class, writer_route)
    effective_prompt_file, prompt_resolution = effective_writer_prompt(
        task_id,
        writer_task_class,
        writer_route,
        prompt_file,
        output_dir,
    )
    clear_analysis_receipt(task_id, writer_task_class)
    clear_analysis_blocker(task_id, writer_task_class)
    manifest: dict[str, Any] = {
        "generated_at": now_utc(),
        "task_id": task_id,
        "writer_task_class": writer_task_class,
        "writer_route_receipt": route_receipt_payload(writer_route),
        "writer_route_receipt_path": str(writer_route_receipt_path),
        "analysis_plan": analysis_plan,
        "prompt_resolution": prompt_resolution,
        "effective_prompt_file": str(effective_prompt_file),
        "analysis_receipt_path": str(analysis_receipt_path(task_id, writer_task_class)),
        "issue_contract_path": str(issue_contract_path(task_id)),
        "writer_authorized": False,
        "status": "blocked",
    }

    if analysis_plan.get("required") != "yes":
        clear_analysis_blocker(task_id, writer_task_class)
        manifest["writer_authorized"] = True
        manifest["status"] = "ready_without_analysis"
        write_manifest(manifest_path, manifest)
        print(str(manifest_path))
        return 0

    analysis_task_class = policy_value(analysis_plan.get("route_task_class"), "")
    if not analysis_task_class:
        manifest["status"] = "blocked_missing_analysis_route"
        manifest["analysis_blocker_path"] = str(
            write_analysis_blocker(
                task_id,
                writer_task_class,
                writer_route,
                status="blocked_missing_analysis_route",
                reason="missing_analysis_route_task_class",
                prepare_manifest=manifest,
            )
        )
        write_manifest(manifest_path, manifest)
        print(str(manifest_path))
        return 2

    analysis_dir = output_dir / "analysis"
    analysis_dir.mkdir(parents=True, exist_ok=True)
    analysis_prompt_file = analysis_dir / "analysis.prompt.txt"
    analysis_prompt_file.write_text(
        analysis_prompt_text(
            original_prompt=read_prompt(effective_prompt_file),
            writer_task_class=writer_task_class,
            analysis_task_class=analysis_task_class,
        ),
        encoding="utf-8",
    )
    manifest["analysis_prompt_file"] = str(analysis_prompt_file)
    analysis_command = [
        sys.executable,
        str(Path(__file__).resolve()),
        "ensemble",
        task_id,
        analysis_task_class,
        str(analysis_prompt_file),
        str(analysis_dir),
        str(workdir),
    ]
    completed = subprocess.run(
        analysis_command,
        cwd=str(workdir),
        capture_output=True,
        text=True,
        check=False,
    )
    manifest["analysis_command"] = analysis_command
    manifest["analysis_return_code"] = completed.returncode
    manifest["analysis_stdout"] = (completed.stdout or "").strip()
    manifest["analysis_stderr"] = (completed.stderr or "").strip()
    analysis_manifest_path = Path((completed.stdout or "").strip().splitlines()[-1]) if (completed.stdout or "").strip() else None
    analysis_manifest: dict[str, Any] = {}
    if analysis_manifest_path and analysis_manifest_path.exists():
        try:
            analysis_manifest = json.loads(analysis_manifest_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            analysis_manifest = {}
    manifest["analysis_manifest_path"] = str(analysis_manifest_path) if analysis_manifest_path else ""
    manifest["analysis_manifest"] = analysis_manifest

    if completed.returncode == 0 and analysis_manifest.get("synthesis_ready") is True:
        receipt_path = write_analysis_receipt(task_id, writer_task_class, writer_route, analysis_manifest)
        manifest["analysis_receipt_path"] = str(receipt_path)
        manifest["analysis_blocker_path"] = str(analysis_blocker_path(task_id, writer_task_class))
        issue_contract, issue_contract_error = build_issue_contract_from_analysis_manifest(
            task_id,
            writer_task_class,
            writer_route,
            analysis_manifest,
        )
        manifest["issue_contract"] = issue_contract
        if issue_contract:
            issue_contract_file = write_issue_contract(task_id, issue_contract)
            manifest["issue_contract_path"] = str(issue_contract_file)
        issue_contract_ok, validated_issue_contract, issue_contract_validation_error = validate_issue_contract(
            task_id,
            writer_task_class,
            writer_route,
        )
        if not issue_contract and issue_contract_error:
            manifest["issue_contract_error"] = issue_contract_error
        elif issue_contract_validation_error:
            manifest["issue_contract_error"] = issue_contract_validation_error
        if validated_issue_contract:
            manifest["issue_contract"] = validated_issue_contract
        if issue_contract_ok:
            existing_prompt_text = read_prompt(effective_prompt_file)
            if worker_packet_gate.validate_packet_text(existing_prompt_text):
                rendered_writer_prompt = write_writer_issue_contract_prompt(
                    output_dir,
                    original_prompt=existing_prompt_text,
                    writer_task_class=writer_task_class,
                    issue_contract=validated_issue_contract or issue_contract,
                )
                manifest["effective_prompt_file"] = str(rendered_writer_prompt)
                prompt_resolution = {
                    **prompt_resolution,
                    "writer_packet_mode": "issue_contract_rendered",
                    "writer_packet_file": str(rendered_writer_prompt),
                }
                manifest["prompt_resolution"] = prompt_resolution
            else:
                prompt_resolution = {
                    **prompt_resolution,
                    "writer_packet_mode": "existing_worker_packet",
                    "writer_packet_file": str(effective_prompt_file),
                }
                manifest["prompt_resolution"] = prompt_resolution
            manifest["writer_authorized"] = True
            manifest["status"] = "analysis_ready"
            write_manifest(manifest_path, manifest)
            print(str(manifest_path))
            return 0
        manifest["status"] = "issue_contract_blocked"
        write_manifest(manifest_path, manifest)
        print(str(manifest_path))
        return 2

    manifest["status"] = "analysis_failed"
    blocker_reason = policy_value(
        ((analysis_manifest.get("post_arbitration_merge_summary") or {}).get("tie_break_reason")),
        policy_value(
            ((analysis_manifest.get("merge_summary") or {}).get("tie_break_reason")),
            policy_value(analysis_manifest.get("status"), "analysis_failed"),
        ),
    )
    manifest["analysis_blocker_path"] = str(
        write_analysis_blocker(
            task_id,
            writer_task_class,
            writer_route,
            status="analysis_failed",
            reason=blocker_reason,
            prepare_manifest=manifest,
            analysis_manifest=analysis_manifest,
        )
    )
    write_manifest(manifest_path, manifest)
    print(str(manifest_path))
    return 2


def run_coach_review(argv: list[str]) -> int:
    if len(argv) < 6:
        print(
            "Usage: python3 _vida/scripts/subagent-dispatch.py coach-review <task_id> <writer_task_class> <prompt_file> <output_dir> [workdir]",
            file=sys.stderr,
        )
        return 1
    task_id, writer_task_class, prompt_file_raw, output_dir_raw = argv[2:6]
    workdir = Path(argv[6]).resolve() if len(argv) > 6 else ROOT_DIR
    prompt_file = Path(prompt_file_raw).resolve()
    output_dir = Path(output_dir_raw).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    snapshot, writer_route = route_snapshot(writer_task_class, task_id)
    manifest_path = output_dir / "coach-review.json"
    coach_plan = writer_route.get("coach_plan") or {}
    writer_route_receipt_path = write_route_receipt(task_id, writer_task_class, writer_route)
    prior_attempts = coach_attempt_count(task_id, writer_task_class)
    clear_coach_receipt(task_id, writer_task_class)
    clear_coach_blocker(task_id, writer_task_class)
    clear_rework_handoff(task_id, writer_task_class)
    attempt_count = prior_attempts + 1
    max_coach_passes = int((writer_route.get("route_budget") or {}).get("max_coach_passes", 0) or coach_plan.get("max_passes", 0) or 0)
    original_prompt = read_prompt(prompt_file)
    manifest: dict[str, Any] = {
        "generated_at": now_utc(),
        "task_id": task_id,
        "writer_task_class": writer_task_class,
        "writer_route_receipt": route_receipt_payload(writer_route),
        "writer_route_receipt_path": str(writer_route_receipt_path),
        "coach_plan": coach_plan,
        "coach_receipt_path": str(coach_receipt_path(task_id, writer_task_class)),
        "coach_blocker_path": str(coach_blocker_path(task_id, writer_task_class)),
        "rework_handoff_path": str(rework_handoff_path(task_id, writer_task_class)),
        "attempt_count": attempt_count,
        "max_coach_passes": max_coach_passes,
        "writer_clear_to_verify": False,
        "status": "blocked",
    }

    if coach_plan.get("required") != "yes":
        manifest["writer_clear_to_verify"] = True
        manifest["status"] = "ready_without_coach"
        write_manifest(manifest_path, manifest)
        print(str(manifest_path))
        return 0

    if max_coach_passes > 0 and attempt_count > max_coach_passes:
        manifest["status"] = "coach_pass_cap_exceeded"
        manifest["coach_blocker_path"] = str(
            write_coach_blocker(
                task_id,
                writer_task_class,
                writer_route,
                status="coach_pass_cap_exceeded",
                reason="max_coach_passes_exceeded",
                attempt_count=attempt_count,
            )
        )
        write_manifest(manifest_path, manifest)
        print(str(manifest_path))
        return 2

    coach_task_class = policy_value(coach_plan.get("route_task_class"), "")
    if not coach_task_class:
        manifest["status"] = "blocked_missing_coach_route"
        manifest["coach_blocker_path"] = str(
            write_coach_blocker(
                task_id,
                writer_task_class,
                writer_route,
                status="blocked_missing_coach_route",
                reason="missing_coach_route_task_class",
                attempt_count=attempt_count,
            )
        )
        write_manifest(manifest_path, manifest)
        print(str(manifest_path))
        return 2

    coach_dir = output_dir / "coach"
    coach_dir.mkdir(parents=True, exist_ok=True)
    coach_prompt_file = coach_dir / "coach.prompt.txt"
    coach_total = max(1, len(coach_selected_subagents(coach_plan)))
    coach_prompt_file.write_text(
        coach_prompt_text(
            original_prompt=original_prompt,
            writer_task_class=writer_task_class,
            coach_task_class=coach_task_class,
            coach_slot=1,
            coach_total=coach_total,
        ),
        encoding="utf-8",
    )
    coach_manifest_path, coach_manifest, coach_decision = run_coach_ensemble(
        task_id=task_id,
        writer_task_class=writer_task_class,
        coach_task_class=coach_task_class,
        prompt_file=coach_prompt_file,
        output_dir=coach_dir,
        workdir=workdir,
        snapshot=snapshot,
        route=writer_route,
        coach_plan=coach_plan,
    )
    manifest["coach_command"] = [
        "internal_runtime",
        "coach_ensemble",
        task_id,
        writer_task_class,
        coach_task_class,
    ]
    manifest["coach_return_code"] = 0 if coach_decision.get("approved") is True else 2
    manifest["coach_stdout"] = str(coach_manifest_path)
    manifest["coach_stderr"] = ""
    coach_manifest["manifest_path"] = str(coach_manifest_path)
    manifest["coach_manifest_path"] = str(coach_manifest_path) if coach_manifest_path else ""
    manifest["coach_manifest"] = coach_manifest
    manifest["coach_decision"] = coach_decision

    if coach_manifest.get("synthesis_ready") is True and coach_decision.get("approved") is True:
        receipt_path = write_coach_receipt(
            task_id,
            writer_task_class,
            writer_route,
            coach_manifest,
            coach_decision=coach_decision,
            attempt_count=attempt_count,
        )
        manifest["coach_receipt_path"] = str(receipt_path)
        manifest["coach_blocker_path"] = str(coach_blocker_path(task_id, writer_task_class))
        manifest["writer_clear_to_verify"] = True
        manifest["status"] = "coach_approved"
        write_manifest(manifest_path, manifest)
        print(str(manifest_path))
        return 0

    rework_requested = (
        coach_decision.get("approved") is False
        and coach_decision.get("coach_decision") == "return_for_rework"
        and coach_decision.get("parsed_json") is True
    )
    blocker_status = "return_for_rework" if rework_requested else "coach_failed"
    blocker_reason = policy_value(
        coach_decision.get("reason"),
        policy_value(coach_manifest.get("status"), "coach_review_failed"),
    )
    rework_payload: dict[str, Any] | None = None
    if blocker_status == "return_for_rework":
        rework_payload = write_rework_handoff(
            task_id,
            writer_task_class,
            writer_route,
            original_prompt=original_prompt,
            coach_decision=coach_decision,
            attempt_count=attempt_count,
            max_coach_passes=max_coach_passes,
        )
        manifest["rework_handoff_path"] = rework_payload["path"]
    manifest["status"] = blocker_status
    manifest["coach_blocker_path"] = str(
        write_coach_blocker(
            task_id,
            writer_task_class,
            writer_route,
            status=blocker_status,
            reason=blocker_reason,
            attempt_count=attempt_count,
            coach_manifest=coach_manifest,
            coach_decision=coach_decision,
            rework_handoff_payload=rework_payload,
        )
    )
    write_manifest(manifest_path, manifest)
    print(str(manifest_path))
    return 2


def usage() -> int:
    print(
        "Usage:\n"
        "  python3 _vida/scripts/subagent-dispatch.py subagent <task_id> <task_class> <subagent> <prompt_file> <output_file> [workdir]\n"
        "  python3 _vida/scripts/subagent-dispatch.py ensemble <task_id> <task_class> <prompt_file> <output_dir> [workdir]\n"
        "  python3 _vida/scripts/subagent-dispatch.py prepare-execution <task_id> <writer_task_class> <prompt_file> <output_dir> [workdir]\n"
        "  python3 _vida/scripts/subagent-dispatch.py coach-review <task_id> <writer_task_class> <prompt_file> <output_dir> [workdir]",
        file=sys.stderr,
    )
    return 1


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        return usage()
    cmd = argv[1]
    if cmd == "subagent":
        return run_single(argv)
    if cmd == "ensemble":
        return run_ensemble(argv)
    if cmd == "prepare-execution":
        return run_prepare_execution(argv)
    if cmd == "coach-review":
        return run_coach_review(argv)
    return usage()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
