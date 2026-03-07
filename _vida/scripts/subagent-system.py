#!/usr/bin/env python3
"""Generic subagent-system runtime helper for VIDA."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import shutil
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
STATE_DIR = ROOT_DIR / ".vida" / "state"
INIT_PATH = STATE_DIR / "subagent-init.json"
SCORECARD_PATH = STATE_DIR / "subagent-scorecards.json"
STRATEGY_PATH = STATE_DIR / "subagent-strategy.json"

VIDA_CONFIG_PATH = SCRIPT_DIR / "vida-config.py"
VIDA_CONFIG_SPEC = importlib.util.spec_from_file_location("vida_config_runtime", VIDA_CONFIG_PATH)
if VIDA_CONFIG_SPEC is None or VIDA_CONFIG_SPEC.loader is None:
    raise RuntimeError(f"Unable to load VIDA config helper: {VIDA_CONFIG_PATH}")
vida_config = importlib.util.module_from_spec(VIDA_CONFIG_SPEC)
VIDA_CONFIG_SPEC.loader.exec_module(vida_config)


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def now_utc_dt() -> datetime:
    return datetime.now(timezone.utc)


def parse_utc_timestamp(value: Any) -> datetime | None:
    if not isinstance(value, str) or not value.strip():
        return None
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc)
    except ValueError:
        return None


def next_utc_day_iso() -> str:
    now = now_utc_dt()
    next_day = (now + timedelta(days=1)).date()
    return datetime.combine(next_day, datetime.min.time(), tzinfo=timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def future_utc_iso(*, minutes: int = 0, hours: int = 0) -> str:
    return (now_utc_dt() + timedelta(hours=hours, minutes=minutes)).isoformat(timespec="seconds").replace("+00:00", "Z")


def ensure_state_dir() -> None:
    STATE_DIR.mkdir(parents=True, exist_ok=True)


def load_json(path: Path, default: Any) -> Any:
    if not path.exists():
        return default
    try:
        return json.loads(path.read_text())
    except Exception:
        return default


def migrate_scorecard_bucket(bucket: dict[str, Any]) -> None:
    if not isinstance(bucket, dict):
        return
    if "subagent_state" not in bucket and isinstance(bucket.get("provider_state"), str):
        bucket["subagent_state"] = bucket.pop("provider_state")
    elif "provider_state" in bucket:
        bucket.pop("provider_state", None)


def save_json(path: Path, payload: Any) -> None:
    ensure_state_dir()
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def migrate_legacy_runtime_state(snapshot: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(snapshot, dict):
        return {}
    if "subagents" not in snapshot and isinstance(snapshot.get("providers"), dict):
        snapshot["subagents"] = snapshot.pop("providers")
    scorecards = snapshot.get("scorecards")
    if isinstance(scorecards, dict) and "subagents" not in snapshot and "providers" in snapshot:
        snapshot["subagents"] = snapshot.get("providers", {})
    if isinstance(scorecards, dict):
        for payload in scorecards.values():
            if not isinstance(payload, dict):
                continue
            migrate_scorecard_bucket(payload.get("global", {}))
            for bucket in (payload.get("by_task_class", {}) or {}).values():
                migrate_scorecard_bucket(bucket)
            for bucket in (payload.get("by_domain", {}) or {}).values():
                migrate_scorecard_bucket(bucket)
    return snapshot


def runtime_snapshot(task_id: str | None = None) -> dict[str, Any]:
    snapshot = migrate_legacy_runtime_state(load_json(INIT_PATH, {}))
    if snapshot.get("subagents"):
        scorecards = load_json(SCORECARD_PATH, {"subagents": {}})
        if isinstance(scorecards, dict):
            if "subagents" not in scorecards and isinstance(scorecards.get("providers"), dict):
                scorecards["subagents"] = scorecards.pop("providers")
            for card in (scorecards.get("subagents", {}) or {}).values():
                if not isinstance(card, dict):
                    continue
                migrate_scorecard_bucket(card.get("global", {}))
                for bucket in (card.get("by_task_class", {}) or {}).values():
                    migrate_scorecard_bucket(bucket)
                for bucket in (card.get("by_domain", {}) or {}).values():
                    migrate_scorecard_bucket(bucket)
            snapshot["scorecards"] = scorecards.get("subagents", snapshot.get("scorecards", {}))
        return snapshot
    return init_snapshot(task_id)


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def split_csv(value: Any) -> list[str]:
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


def models_hint_for_subagent(subagent_name: str, subagent_cfg: dict[str, Any]) -> list[str]:
    hint = subagent_cfg.get("models_hint")
    if isinstance(hint, str):
        return split_csv(hint)
    if subagent_name != "codex_cli":
        return []
    cache = Path.home() / ".codex" / "models_cache.json"
    payload = load_json(cache, {})
    out: list[str] = []
    for item in payload.get("models", []):
        slug = item.get("slug")
        if isinstance(slug, str):
            out.append(slug)
    return out


def policy_value(value: Any, default: str) -> str:
    if value is None:
        return default
    if isinstance(value, str):
        trimmed = value.strip()
        return trimmed if trimmed else default
    return str(value)


def policy_int(value: Any, default: int) -> int:
    if value is None:
        return default
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def inferred_risk_class(task_class: str, write_scope: str, verification_gate: str) -> str:
    normalized_scope = policy_value(write_scope, "none")
    normalized_gate = policy_value(verification_gate, "subagent_return_contract")
    normalized_task = policy_value(task_class, "default")
    if normalized_scope in {"orchestrator_native", "external_write", "repo_write"}:
        return "R3"
    if normalized_scope in {"scoped_only", "sandbox", "patch"}:
        return "R2"
    if normalized_gate in {"architectural_review", "targeted_verification"}:
        return "R1"
    if normalized_task in {"architecture"}:
        return "R1"
    return "R0"


def detect_subagents(config: dict[str, Any]) -> dict[str, Any]:
    subagents = vida_config.dotted_get(config, "agent_system.subagents", {}) or {}
    detected: dict[str, Any] = {}
    for name, subagent_cfg in subagents.items():
        if not isinstance(subagent_cfg, dict):
            continue
        enabled = bool(subagent_cfg.get("enabled", False))
        subagent_backend_class = subagent_cfg.get("subagent_backend_class", "external_cli")
        detect_command = subagent_cfg.get("detect_command")
        if name == "internal_subagents":
            available = enabled
            reason = "runtime-managed"
        else:
            if not isinstance(detect_command, str) or not detect_command:
                detect_command = name.replace("_cli", "")
            available = enabled and shutil.which(detect_command) is not None
            reason = f"command:{detect_command}"
        detected[name] = {
            "enabled": enabled,
            "available": available,
            "subagent_backend_class": subagent_backend_class,
            "role": subagent_cfg.get("role", "secondary"),
            "orchestration_tier": policy_value(subagent_cfg.get("orchestration_tier"), "standard"),
            "cost_priority": policy_value(subagent_cfg.get("cost_priority"), "normal"),
            "detect_command": detect_command,
            "models_hint": models_hint_for_subagent(name, subagent_cfg),
            "default_model": subagent_cfg.get("default_model"),
            "profiles": split_csv(subagent_cfg.get("profiles")),
            "default_profile": subagent_cfg.get("default_profile"),
            "capability_band": split_csv(subagent_cfg.get("capability_band")),
            "write_scope": policy_value(subagent_cfg.get("write_scope"), "none"),
            "billing_tier": policy_value(subagent_cfg.get("billing_tier"), "unknown"),
            "speed_tier": policy_value(subagent_cfg.get("speed_tier"), "unknown"),
            "quality_tier": policy_value(subagent_cfg.get("quality_tier"), "unknown"),
            "specialties": split_csv(subagent_cfg.get("specialties")),
            "dispatch": subagent_cfg.get("dispatch", {}) if isinstance(subagent_cfg.get("dispatch"), dict) else {},
            "reason": reason,
        }
    return detected


def thresholds(config: dict[str, Any]) -> dict[str, int]:
    scoring = vida_config.dotted_get(config, "agent_system.scoring", {}) or {}
    return {
        "consecutive_failure_limit": int(scoring.get("consecutive_failure_limit", 5)),
        "promotion_score": int(scoring.get("promotion_score", 80)),
        "demotion_score": int(scoring.get("demotion_score", 35)),
    }


def effective_mode(config: dict[str, Any], subagents: dict[str, Any]) -> tuple[str, list[str]]:
    protocol_active = bool(vida_config.dotted_get(config, "protocol_activation.agent_system", False))
    if not protocol_active:
        return "disabled", ["protocol_activation.agent_system=false"]

    requested = str(vida_config.dotted_get(config, "agent_system.mode", "native"))
    has_internal = bool(subagents.get("internal_subagents", {}).get("available"))
    has_external = any(
        name != "internal_subagents" and payload.get("available")
        for name, payload in subagents.items()
    )

    if requested == "disabled":
        return "disabled", ["requested_mode=disabled"]
    if requested == "native":
        if has_internal:
            return "native", ["requested_mode=native"]
        return "disabled", ["requested_mode=native", "internal_subagents unavailable"]
    if requested == "hybrid":
        if has_internal and has_external:
            return "hybrid", ["requested_mode=hybrid"]
        if has_internal:
            return "native", ["requested_mode=hybrid", "external subagents unavailable -> degrade_to=native"]
        if has_external:
            return "disabled", ["requested_mode=hybrid", "internal subagents unavailable -> degrade_to=disabled"]
        return "disabled", ["requested_mode=hybrid", "no subagents available"]
    return "disabled", [f"unsupported requested_mode={requested}"]


def score_defaults() -> dict[str, Any]:
    return {
        "global": {
            "score": 50,
            "success_count": 0,
            "failure_count": 0,
            "consecutive_failures": 0,
            "state": "normal",
            "useful_progress_count": 0,
            "chatter_only_count": 0,
            "timeout_after_progress_count": 0,
            "time_to_first_useful_output_samples": 0,
            "avg_time_to_first_useful_output_ms": 0,
            "useful_progress_rate": 0,
            "subagent_state": "active",
            "failure_reason": "",
            "cooldown_until": "",
            "probe_required": False,
            "last_quota_exhausted_at": "",
        },
        "by_task_class": {},
        "by_domain": {},
    }


def should_degrade_for_chatter(bucket: dict[str, Any]) -> bool:
    chatter_only_count = int(bucket.get("chatter_only_count", 0) or 0)
    useful_progress_count = int(bucket.get("useful_progress_count", 0) or 0)
    success_count = int(bucket.get("success_count", 0) or 0)
    useful_progress_rate = float(bucket.get("useful_progress_rate", 0) or 0)
    subagent_state = policy_value(bucket.get("subagent_state"), "active")
    failure_reason = policy_value(bucket.get("failure_reason"), "")
    if subagent_state in {"quota_exhausted", "disabled_manual"}:
        return False
    if failure_reason in {
        "daily_quota_exhausted",
        "rate_limited",
        "auth_invalid",
        "interactive_blocked",
        "runtime_unstable",
    }:
        return False
    return (
        chatter_only_count >= 2
        and useful_progress_count == 0
        and success_count == 0
        and useful_progress_rate <= 0
    )


def apply_behavioral_degradation(bucket: dict[str, Any]) -> None:
    if not should_degrade_for_chatter(bucket):
        return
    bucket["subagent_state"] = "degraded"
    bucket["failure_reason"] = "repeated_chatter_only"
    bucket["cooldown_until"] = future_utc_iso(minutes=30)
    bucket["probe_required"] = True


def normalize_availability_bucket(bucket: dict[str, Any]) -> None:
    if not isinstance(bucket, dict):
        return
    state = policy_value(bucket.get("subagent_state"), "active")
    failure_reason = policy_value(bucket.get("failure_reason"), "")
    if state == "degraded" and failure_reason == "auth_invalid":
        bucket["probe_required"] = True
    if state == "degraded" and failure_reason == "interactive_blocked":
        bucket["probe_required"] = True


def availability_active(bucket: dict[str, Any]) -> bool:
    apply_behavioral_degradation(bucket)
    normalize_availability_bucket(bucket)
    state = policy_value(bucket.get("subagent_state"), "active")
    failure_reason = policy_value(bucket.get("failure_reason"), "")
    cooldown_until = parse_utc_timestamp(bucket.get("cooldown_until"))
    now = now_utc_dt()
    if cooldown_until is not None and cooldown_until > now:
        return False
    if state == "quota_exhausted":
        return cooldown_until is not None and cooldown_until <= now
    if state == "disabled_manual":
        return False
    if state == "degraded" and failure_reason in {"auth_invalid", "interactive_blocked"}:
        return False
    return True


def apply_availability_metrics(bucket: dict[str, Any], result: str, metrics: dict[str, Any]) -> None:
    subagent_state = policy_value(metrics.get("subagent_state"), "")
    failure_reason = policy_value(metrics.get("failure_reason"), "")
    cooldown_until = policy_value(metrics.get("cooldown_until"), "")
    probe_required = bool(metrics.get("probe_required", False))
    quota_exhausted_at = policy_value(metrics.get("last_quota_exhausted_at"), "")

    if result == "success":
        bucket["subagent_state"] = "active"
        bucket["failure_reason"] = ""
        bucket["cooldown_until"] = ""
        bucket["probe_required"] = False
        apply_behavioral_degradation(bucket)
        normalize_availability_bucket(bucket)
        return

    if subagent_state:
        bucket["subagent_state"] = subagent_state
    elif not policy_value(bucket.get("subagent_state"), ""):
        bucket["subagent_state"] = "degraded"
    if failure_reason:
        bucket["failure_reason"] = failure_reason
    if cooldown_until:
        bucket["cooldown_until"] = cooldown_until
    if quota_exhausted_at:
        bucket["last_quota_exhausted_at"] = quota_exhausted_at
    bucket["probe_required"] = probe_required
    apply_behavioral_degradation(bucket)
    normalize_availability_bucket(bucket)


def update_subagent_availability(subagent: str, metrics: dict[str, Any], note: str = "") -> dict[str, Any]:
    snapshot = runtime_snapshot()
    scorecards = load_json(SCORECARD_PATH, {"subagents": {}})
    subagent_cards = scorecards.setdefault("subagents", {})
    card = subagent_cards.setdefault(subagent, score_defaults())
    global_card = card.setdefault("global", score_defaults()["global"])
    apply_availability_metrics(global_card, "success" if metrics.get("subagent_state") == "active" else "failure", metrics)
    global_card["last_probe_note"] = note
    global_card["last_probe_at"] = now_utc()
    save_json(SCORECARD_PATH, scorecards)
    snapshot["scorecards"] = scorecards["subagents"]
    snapshot["written_at"] = now_utc()
    save_json(INIT_PATH, snapshot)
    return {"subagent": subagent, "availability": global_card}


def subagent_operator_status() -> dict[str, Any]:
    snapshot = runtime_snapshot()
    subagents = snapshot.get("subagents", {})
    scorecards = snapshot.get("scorecards", {})
    rows: list[dict[str, Any]] = []
    for subagent_name, subagent_cfg in subagents.items():
        subagent_scorecard = scorecards.get(subagent_name, {})
        global_card = subagent_scorecard.get("global", {})
        apply_behavioral_degradation(global_card)
        normalize_availability_bucket(global_card)
        subagent_state = policy_value(global_card.get("subagent_state"), "active")
        cooldown_until = policy_value(global_card.get("cooldown_until"), "")
        probe_required = bool(global_card.get("probe_required", False))
        failure_reason = policy_value(global_card.get("failure_reason"), "")
        score = int(global_card.get("score", 50))
        success_count = int(global_card.get("success_count", 0))
        failure_count = int(global_card.get("failure_count", 0))
        chatter_only_count = int(global_card.get("chatter_only_count", 0))
        useful_progress_rate = float(global_card.get("useful_progress_rate", 0) or 0)
        task_class_cards = subagent_scorecard.get("by_task_class", {}) or {}
        preferred_task_classes = sorted(
            [
                task_class
                for task_class, bucket in task_class_cards.items()
                if isinstance(bucket, dict) and policy_value(bucket.get("state"), "normal") == "preferred"
            ]
        )
        row = {
            "subagent": subagent_name,
            "available": bool(subagent_cfg.get("available", False)),
            "subagent_state": subagent_state,
            "failure_reason": failure_reason,
            "cooldown_until": cooldown_until,
            "probe_required": probe_required,
            "score": score,
            "state": policy_value(global_card.get("state"), "normal"),
            "success_count": success_count,
            "failure_count": failure_count,
            "chatter_only_count": chatter_only_count,
            "useful_progress_rate": useful_progress_rate,
            "quality_tier": policy_value(subagent_cfg.get("quality_tier"), "unknown"),
            "billing_tier": policy_value(subagent_cfg.get("billing_tier"), "unknown"),
            "preferred_task_classes": preferred_task_classes,
            "recommended_action": (
                "wait_for_cooldown"
                if cooldown_until
                else "repair_auth_then_probe"
                if failure_reason == "auth_invalid"
                else "fix_headless_profile_then_probe"
                if failure_reason == "interactive_blocked"
                else "run_probe"
                if probe_required
                else "reduce_prompt_scope"
                if failure_reason == "repeated_chatter_only"
                else "healthy"
                if subagent_state == "active"
                else "investigate"
            ),
        }
        rows.append(row)
    rows.sort(
        key=lambda item: (
            item["recommended_action"] != "healthy",
            item["subagent_state"] != "active",
            -int(item["score"]),
            item["subagent"],
        )
    )
    return {
        "generated_at": now_utc(),
        "subagents": rows,
        "summary": {
            "healthy": sum(1 for row in rows if row["subagent_state"] == "active" and not row["probe_required"] and not row["cooldown_until"]),
            "cooldown": sum(1 for row in rows if row["cooldown_until"]),
            "probe_required": sum(1 for row in rows if row["probe_required"]),
            "degraded": sum(1 for row in rows if row["subagent_state"] == "degraded"),
            "quota_exhausted": sum(1 for row in rows if row["subagent_state"] == "quota_exhausted"),
        },
    }


def subagent_probe_command(subagent: str, subagent_cfg: dict[str, Any]) -> tuple[list[str], int, str]:
    dispatch_cfg = subagent_cfg.get("dispatch", {})
    command = policy_value(dispatch_cfg.get("command"), "")
    if not command:
        raise ValueError(f"cli subagent {subagent} missing dispatch.command")
    static_args = split_csv(dispatch_cfg.get("probe_static_args")) or split_csv(dispatch_cfg.get("static_args"))
    prompt = policy_value(dispatch_cfg.get("probe_prompt"), "Return exactly one line: VIDA_CLI_SUBAGENT_OK")
    prompt_mode = policy_value(dispatch_cfg.get("prompt_mode"), "positional")
    prompt_flag = policy_value(dispatch_cfg.get("prompt_flag"), "")
    timeout_seconds = max(5, policy_int(dispatch_cfg.get("probe_timeout_seconds"), 15))
    cmd = [command, *static_args]
    if prompt_mode == "flag":
        if not prompt_flag:
            raise ValueError(f"cli subagent {subagent} probe requires dispatch.prompt_flag for flag mode")
        cmd.extend([prompt_flag, prompt])
    else:
        cmd.append(prompt)
    return cmd, timeout_seconds, policy_value(dispatch_cfg.get("probe_expect_substring"), "")


def availability_signal_for_probe(result: str, combined_text: str) -> dict[str, Any]:
    text = (combined_text or "").lower()
    if result == "success":
        return {
            "subagent_state": "active",
            "failure_reason": "",
            "cooldown_until": "",
            "probe_required": False,
            "last_quota_exhausted_at": "",
        }
    if any(marker in text for marker in ("quota exceeded", "quota exhausted", "daily quota", "daily limit", "try again tomorrow", "usage limit reached for today")):
        now_ts = now_utc()
        return {
            "subagent_state": "quota_exhausted",
            "failure_reason": "daily_quota_exhausted",
            "cooldown_until": next_utc_day_iso(),
            "probe_required": True,
            "last_quota_exhausted_at": now_ts,
        }
    if any(marker in text for marker in ("rate limit", "too many requests", "429", "requests per minute")):
        return {
            "subagent_state": "degraded",
            "failure_reason": "rate_limited",
            "cooldown_until": future_utc_iso(minutes=30),
            "probe_required": True,
            "last_quota_exhausted_at": "",
        }
    if any(marker in text for marker in ("invalid api key", "authentication failed", "unauthorized", "invalid credentials", "permission denied")):
        return {
            "subagent_state": "degraded",
            "failure_reason": "auth_invalid",
            "cooldown_until": "",
            "probe_required": True,
            "last_quota_exhausted_at": "",
        }
    if any(marker in text for marker in ("approval mode", "interactive mode", "requires interactive", "stdin is not a tty", "prompt for approval")):
        return {
            "subagent_state": "degraded",
            "failure_reason": "interactive_blocked",
            "cooldown_until": future_utc_iso(hours=12),
            "probe_required": True,
            "last_quota_exhausted_at": "",
        }
    return {
        "subagent_state": "degraded",
        "failure_reason": "runtime_unstable",
        "cooldown_until": future_utc_iso(minutes=30),
        "probe_required": True,
        "last_quota_exhausted_at": "",
    }


def probe_subagent(subagent: str) -> dict[str, Any]:
    config = vida_config.load_validated_config()
    subagents = detect_subagents(config)
    subagent_cfg = subagents.get(subagent)
    if not subagent_cfg:
        raise ValueError(f"unknown cli subagent: {subagent}")
    if not subagent_cfg.get("available"):
        metrics = {
            "subagent_state": "degraded",
            "failure_reason": "detect_command_missing",
            "cooldown_until": "",
            "probe_required": True,
            "last_quota_exhausted_at": "",
        }
        return update_subagent_availability(subagent, metrics, "probe failed: detect command unavailable")

    cmd, timeout_seconds, expect_substring = subagent_probe_command(subagent, subagent_cfg)
    probe_dir = ROOT_DIR / "_temp" / "subagent-probes"
    ensure_parent(probe_dir / ".keep")
    stdout_path = probe_dir / f"{subagent}.stdout.log"
    stderr_path = probe_dir / f"{subagent}.stderr.log"
    try:
        completed = subprocess.run(
            cmd,
            cwd=str(ROOT_DIR),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout_seconds,
            check=False,
            env={**os.environ.copy(), **subagent_cfg.get("dispatch", {}).get("env", {})},
        )
        stdout_text = completed.stdout or ""
        stderr_text = completed.stderr or ""
        stdout_path.write_text(stdout_text, encoding="utf-8")
        stderr_path.write_text(stderr_text, encoding="utf-8")
        success = completed.returncode == 0 and (not expect_substring or expect_substring in stdout_text or expect_substring in stderr_text)
        metrics = availability_signal_for_probe("success" if success else "failure", "\n".join([stdout_text, stderr_text]))
        note = f"probe exit={completed.returncode}; expect={expect_substring or '<non-empty/zero-exit>'}"
        result = update_subagent_availability(subagent, metrics, note)
        result["probe"] = {
            "command": cmd,
            "timeout_seconds": timeout_seconds,
            "stdout_file": str(stdout_path),
            "stderr_file": str(stderr_path),
            "success": success,
        }
        return result
    except subprocess.TimeoutExpired:
        stdout_path.write_text("", encoding="utf-8")
        stderr_path.write_text(f"subagent probe timed out after {timeout_seconds}s\n", encoding="utf-8")
        metrics = availability_signal_for_probe("failure", "subagent probe timed out")
        result = update_subagent_availability(subagent, metrics, f"probe timeout after {timeout_seconds}s")
        result["probe"] = {
            "command": cmd,
            "timeout_seconds": timeout_seconds,
            "stdout_file": str(stdout_path),
            "stderr_file": str(stderr_path),
            "success": False,
        }
        return result


def recover_subagent(subagent: str) -> dict[str, Any]:
    status = subagent_operator_status()
    row = next((item for item in status.get("subagents", []) if item.get("subagent") == subagent), None)
    if row is None:
        raise ValueError(f"unknown cli subagent: {subagent}")
    cooldown_until = parse_utc_timestamp(row.get("cooldown_until"))
    if cooldown_until is not None and cooldown_until > now_utc_dt():
        return {
            "subagent": subagent,
            "status": "blocked",
            "reason": "cooldown_active",
            "cooldown_until": row.get("cooldown_until", ""),
            "recommended_action": row.get("recommended_action", "wait_for_cooldown"),
        }
    if not row.get("probe_required") and row.get("subagent_state") == "active":
        return {
            "subagent": subagent,
            "status": "noop",
            "reason": "already_healthy",
            "recommended_action": "none",
        }
    result = probe_subagent(subagent)
    result["status"] = "recovered" if result.get("availability", {}).get("subagent_state") == "active" else "still_degraded"
    return result


def recover_pending_subagents() -> dict[str, Any]:
    status = subagent_operator_status()
    results: list[dict[str, Any]] = []
    for row in status.get("subagents", []):
        subagent = str(row.get("subagent", ""))
        if not subagent:
            continue
        cooldown_until = parse_utc_timestamp(row.get("cooldown_until"))
        if cooldown_until is not None and cooldown_until > now_utc_dt():
            continue
        if row.get("probe_required") or row.get("subagent_state") in {"degraded", "quota_exhausted"}:
            results.append(recover_subagent(subagent))
    return {
        "generated_at": now_utc(),
        "results": results,
        "attempted": len(results),
    }


def update_average(current_avg: int, current_samples: int, new_value: int) -> tuple[int, int]:
    samples = current_samples + 1
    avg = int(round(((current_avg * current_samples) + new_value) / samples))
    return avg, samples


def load_scorecards(subagents: dict[str, Any]) -> dict[str, Any]:
    payload = load_json(SCORECARD_PATH, {"subagents": {}})
    if "subagents" not in payload and isinstance(payload.get("providers"), dict):
        payload["subagents"] = payload.pop("providers")
    subagent_payload = payload.setdefault("subagents", {})
    for name in subagents:
        subagent_payload.setdefault(name, score_defaults())
    for card in subagent_payload.values():
        if not isinstance(card, dict):
            continue
        migrate_scorecard_bucket(card.get("global", {}))
        for bucket in (card.get("by_task_class", {}) or {}).values():
            migrate_scorecard_bucket(bucket)
        for bucket in (card.get("by_domain", {}) or {}).values():
            migrate_scorecard_bucket(bucket)
    return payload


def classify_state(score: int, consecutive_failures: int, cfg: dict[str, int]) -> str:
    if consecutive_failures >= cfg["consecutive_failure_limit"]:
        return "demoted"
    if score >= cfg["promotion_score"]:
        return "preferred"
    return "normal"


def init_snapshot(task_id: str | None = None) -> dict[str, Any]:
    config = vida_config.load_validated_config()
    subagent_state = detect_subagents(config)
    scoring_cfg = thresholds(config)
    mode, reasons = effective_mode(config, subagent_state)
    scorecards = load_scorecards(subagent_state)
    snapshot = {
        "written_at": now_utc(),
        "task_id": task_id,
        "config_path": str(vida_config.CONFIG_PATH) if vida_config.CONFIG_PATH.exists() else "",
        "protocol_activation": {
            "agent_system": bool(vida_config.dotted_get(config, "protocol_activation.agent_system", False)),
        },
        "agent_system": {
            "init_on_boot": bool(vida_config.dotted_get(config, "agent_system.init_on_boot", False)),
            "requested_mode": str(vida_config.dotted_get(config, "agent_system.mode", "native")),
            "effective_mode": mode,
            "state_owner": str(vida_config.dotted_get(config, "agent_system.state_owner", "orchestrator_only")),
            "max_parallel_agents": int(vida_config.dotted_get(config, "agent_system.max_parallel_agents", 1)),
            "scoring": scoring_cfg,
            "reasons": reasons,
        },
        "subagents": subagent_state,
        "scorecards": scorecards["subagents"],
    }
    save_json(INIT_PATH, snapshot)
    save_json(SCORECARD_PATH, scorecards)
    return snapshot


def route_config_for(config: dict[str, Any], task_class: str) -> dict[str, Any]:
    routing = vida_config.dotted_get(config, f"agent_system.routing.{task_class}", {})
    if isinstance(routing, dict):
        return routing
    return {}


def route_models(route_cfg: dict[str, Any]) -> dict[str, str]:
    models = route_cfg.get("models", {})
    if not isinstance(models, dict):
        return {}
    return {str(name): str(model) for name, model in models.items() if model}


def route_profiles(route_cfg: dict[str, Any]) -> dict[str, str]:
    profiles = route_cfg.get("profiles", {})
    if not isinstance(profiles, dict):
        return {}
    return {str(name): str(profile) for name, profile in profiles.items() if profile}


def selected_model_for_subagent(subagent: str, subagent_cfg: dict[str, Any], route_cfg: dict[str, Any]) -> tuple[str | None, str]:
    route_model = route_models(route_cfg).get(subagent)
    if route_model:
        return route_model, "route_override"
    default_model = subagent_cfg.get("default_model")
    if isinstance(default_model, str) and default_model:
        return default_model, "subagent_default"
    return None, "none"


def selected_profile_for_subagent(subagent: str, subagent_cfg: dict[str, Any], route_cfg: dict[str, Any]) -> tuple[str | None, str]:
    route_profile = route_profiles(route_cfg).get(subagent)
    available_profiles = subagent_cfg.get("profiles", [])
    if route_profile and route_profile in available_profiles:
        return route_profile, "route_override"
    default_profile = subagent_cfg.get("default_profile")
    if isinstance(default_profile, str) and default_profile:
        return default_profile, "subagent_default"
    return None, "none"


def adaptive_runtime_seconds(
    base_limit: int,
    subagent_cfg: dict[str, Any],
    task_card: dict[str, Any],
    global_card: dict[str, Any],
    effective_score: int,
) -> int:
    subagent_limit = policy_int(subagent_cfg.get("max_runtime_seconds"), 0)
    baseline = base_limit or subagent_limit or 180
    latency_ms = policy_int(task_card.get("last_latency_ms"), 0) or policy_int(global_card.get("last_latency_ms"), 0)
    ttfu_ms = (
        policy_int(task_card.get("avg_time_to_first_useful_output_ms"), 0)
        or policy_int(global_card.get("avg_time_to_first_useful_output_ms"), 0)
    )
    useful_progress_rate = float(
        task_card.get("useful_progress_rate", global_card.get("useful_progress_rate", 0)) or 0
    )
    chatter_only_count = int(
        task_card.get("chatter_only_count", global_card.get("chatter_only_count", 0)) or 0
    )
    timeout_after_progress_count = int(
        task_card.get("timeout_after_progress_count", global_card.get("timeout_after_progress_count", 0)) or 0
    )
    last_result = policy_value(task_card.get("last_result"), policy_value(global_card.get("last_result"), ""))
    quality_tier = policy_value(subagent_cfg.get("quality_tier"), "medium")
    speed_tier = policy_value(subagent_cfg.get("speed_tier"), "medium")

    budget = baseline
    if latency_ms > 0 and last_result == "success":
        learned_seconds = int((latency_ms / 1000.0) * 1.2) + 20
        budget = max(budget, learned_seconds)
    elif latency_ms > 0 and last_result == "failure":
        budget = max(budget, int((latency_ms / 1000.0) * 0.9))

    if ttfu_ms > 0:
        budget = max(budget, int((ttfu_ms / 1000.0) * 1.15) + 25)

    if quality_tier == "high" and effective_score >= 65:
        budget += 20
    if useful_progress_rate >= 0.25:
        budget += 15
    if timeout_after_progress_count > 0:
        budget -= min(20, timeout_after_progress_count * 10)
    if speed_tier == "fast":
        budget = min(budget, baseline + 20)

    return max(120, min(300, budget))


def task_class_fit_bonus(task_class: str, subagent_cfg: dict[str, Any]) -> tuple[int, list[str]]:
    normalized_task = policy_value(task_class, "default").casefold()
    specialties = {item.casefold() for item in split_csv(subagent_cfg.get("specialties"))}
    capability_band = {item.casefold() for item in split_csv(subagent_cfg.get("capability_band"))}
    bonus = 0
    reasons: list[str] = []

    direct_specialty_map = {
        "analysis": {"review", "research", "planning", "spec"},
        "review": {"review", "deep_review"},
        "research": {"research", "long_context"},
        "verification": {"review", "deep_review"},
        "meta_analysis": {"architecture", "research", "review"},
        "implementation": {"implementation", "code_edit", "bounded_write"},
        "architecture": {"architecture", "integration"},
    }
    direct_capability_map = {
        "analysis": {"read_only", "review_safe"},
        "review": {"review_safe"},
        "research": {"read_only"},
        "verification": {"review_safe"},
        "implementation": {"bounded_write_safe", "implementation_safe"},
        "architecture": {"architecture_safe"},
    }

    matched_specialties = sorted(specialties & direct_specialty_map.get(normalized_task, set()))
    matched_capabilities = sorted(capability_band & direct_capability_map.get(normalized_task, set()))
    if matched_specialties:
        bonus += min(12, 4 * len(matched_specialties))
        reasons.append(f"specialty:{','.join(matched_specialties[:3])}")
    if matched_capabilities:
        bonus += min(8, 4 * len(matched_capabilities))
        reasons.append(f"capability:{','.join(matched_capabilities[:3])}")
    return bonus, reasons


def apply_bridge_fallback_priority(
    candidates: list[dict[str, Any]],
    bridge_fallback_subagent: str,
) -> list[dict[str, Any]]:
    if not bridge_fallback_subagent or len(candidates) < 2:
        return candidates
    bridge_index = next(
        (idx for idx, item in enumerate(candidates) if item.get("subagent") == bridge_fallback_subagent),
        -1,
    )
    if bridge_index <= 1:
        return candidates
    bridge_item = candidates[bridge_index]
    remaining = [item for idx, item in enumerate(candidates) if idx != bridge_index]
    selected = remaining[:1]
    tail = remaining[1:]
    internal_items = [item for item in tail if item.get("subagent_backend_class") == "internal"]
    external_items = [item for item in tail if item.get("subagent_backend_class") != "internal"]
    return [*selected, bridge_item, *external_items, *internal_items]


def route_subagent(task_class: str) -> dict[str, Any]:
    snapshot = runtime_snapshot()
    if snapshot.get("agent_system", {}).get("effective_mode") == "disabled":
        return {
            "task_class": task_class,
            "selected_subagent": None,
            "reason": "subagent system disabled",
            "effective_score": 0,
        }

    config = vida_config.load_validated_config()
    task_route_cfg = route_config_for(config, task_class)
    subagent_order = split_csv(task_route_cfg.get("subagents"))
    if not subagent_order:
        subagent_order = split_csv(vida_config.dotted_get(config, "agent_system.routing.default.subagents", ""))
    subagents = snapshot.get("subagents", {})
    scores = snapshot.get("scorecards", {})
    scoring_cfg = snapshot.get("agent_system", {}).get("scoring", thresholds(config))
    mode = snapshot.get("agent_system", {}).get("effective_mode")
    max_parallel_agents = int(snapshot.get("agent_system", {}).get("max_parallel_agents", 1))
    state_owner = str(snapshot.get("agent_system", {}).get("state_owner", "orchestrator_only"))
    write_scope = policy_value(task_route_cfg.get("write_scope"), "none")
    verification_gate = policy_value(task_route_cfg.get("verification_gate"), "subagent_return_contract")
    risk_class = inferred_risk_class(task_class, write_scope, verification_gate)
    max_runtime_seconds = policy_int(task_route_cfg.get("max_runtime_seconds"), 0)
    min_output_bytes = policy_int(task_route_cfg.get("min_output_bytes"), 0)
    merge_policy = policy_value(task_route_cfg.get("merge_policy"), "single_subagent")
    fanout_order = split_csv(task_route_cfg.get("fanout_subagents"))
    dispatch_required = policy_value(task_route_cfg.get("dispatch_required"), "optional")
    external_first_required = policy_value(task_route_cfg.get("external_first_required"), "no")
    bridge_fallback_subagent = policy_value(task_route_cfg.get("bridge_fallback_subagent"), "")
    internal_escalation_trigger = policy_value(task_route_cfg.get("internal_escalation_trigger"), "")

    candidates: list[dict[str, Any]] = []
    suppressed_subagents: list[dict[str, Any]] = []
    for idx, subagent in enumerate(subagent_order):
        payload = subagents.get(subagent, {})
        if not payload.get("enabled"):
            suppressed_subagents.append({"subagent": subagent, "reason": "disabled"})
            continue
        if not payload.get("available"):
            suppressed_subagents.append({"subagent": subagent, "reason": "detect_command_unavailable"})
            continue
        if mode == "native" and payload.get("subagent_backend_class") != "internal":
            suppressed_subagents.append({"subagent": subagent, "reason": "native_mode_external_filtered"})
            continue
        card = scores.get(subagent, score_defaults())
        global_card = card.get("global", {})
        if not availability_active(global_card):
            suppressed_subagents.append(
                {
                    "subagent": subagent,
                    "reason": "availability_blocked",
                    "subagent_state": policy_value(global_card.get("subagent_state"), "unknown"),
                    "failure_reason": policy_value(global_card.get("failure_reason"), ""),
                    "cooldown_until": policy_value(global_card.get("cooldown_until"), ""),
                    "probe_required": bool(global_card.get("probe_required", False)),
                }
            )
            continue
        task_card = card.get("by_task_class", {}).get(task_class, {})
        learned_score = int(task_card.get("score", card.get("global", {}).get("score", 50)))
        global_score = int(global_card.get("score", 50))
        useful_progress_rate = float(
            task_card.get("useful_progress_rate", global_card.get("useful_progress_rate", 0)) or 0
        )
        chatter_only_count = int(
            task_card.get("chatter_only_count", global_card.get("chatter_only_count", 0)) or 0
        )
        timeout_after_progress_count = int(
            task_card.get("timeout_after_progress_count", global_card.get("timeout_after_progress_count", 0)) or 0
        )
        state = task_card.get("state", card.get("global", {}).get("state", "normal"))
        subagent_state = policy_value(global_card.get("subagent_state"), "active")
        consecutive = int(task_card.get("consecutive_failures", card.get("global", {}).get("consecutive_failures", 0)))
        if state == "demoted" and consecutive >= int(scoring_cfg["consecutive_failure_limit"]):
            continue
        priority_bonus = max(0, 30 - (idx * 10))
        progress_bonus = int(round(useful_progress_rate * 20))
        chatter_penalty = min(20, chatter_only_count * 10)
        timeout_penalty = min(15, timeout_after_progress_count * 8)
        fit_bonus, fit_reasons = task_class_fit_bonus(task_class, payload)
        effective_score = learned_score + priority_bonus + progress_bonus + fit_bonus - chatter_penalty - timeout_penalty
        selected_model, model_source = selected_model_for_subagent(subagent, payload, task_route_cfg)
        selected_profile, profile_source = selected_profile_for_subagent(subagent, payload, task_route_cfg)
        candidate_runtime = adaptive_runtime_seconds(
            max_runtime_seconds,
            payload,
            task_card,
            global_card,
            effective_score,
        )
        candidates.append(
            {
                "effective_score": effective_score,
                "subagent": subagent,
                "state": state,
                "task_fit_score": learned_score,
                "global_score": global_score,
                "task_class_fit_bonus": fit_bonus,
                "task_class_fit_reasons": fit_reasons,
                "success_count": int(task_card.get("success_count", global_card.get("success_count", 0)) or 0),
                "selected_model": selected_model,
                "selected_model_source": model_source,
                "selected_profile": selected_profile,
                "selected_profile_source": profile_source,
                "max_runtime_seconds": candidate_runtime,
                "subagent_backend_class": payload.get("subagent_backend_class"),
                "subagent_state": subagent_state,
                "capability_band": payload.get("capability_band", []),
                "subagent_write_scope": payload.get("write_scope", "none"),
                "orchestration_tier": payload.get("orchestration_tier", "standard"),
                "cost_priority": payload.get("cost_priority", "normal"),
                "useful_progress_rate": useful_progress_rate,
                "chatter_only_count": chatter_only_count,
                "timeout_after_progress_count": timeout_after_progress_count,
            }
        )

    if not candidates:
        return {
            "task_class": task_class,
            "selected_subagent": None,
            "reason": "no eligible subagents after mode/capability/score filtering",
            "effective_score": 0,
            "selected_model": None,
            "selected_model_source": "none",
            "selected_profile": None,
            "selected_profile_source": "none",
            "fallback_subagents": [],
            "write_scope": write_scope,
            "verification_gate": verification_gate,
            "risk_class": risk_class,
            "max_runtime_seconds": max_runtime_seconds,
            "min_output_bytes": min_output_bytes,
            "fanout_subagents": [],
            "fanout_min_results": 0,
            "merge_policy": merge_policy,
            "dispatch_required": dispatch_required,
            "external_first_required": external_first_required,
            "bridge_fallback_subagent": bridge_fallback_subagent,
            "internal_escalation_trigger": internal_escalation_trigger,
            "max_parallel_agents": max_parallel_agents,
            "state_owner": state_owner,
            "suppressed_subagents": suppressed_subagents,
        }

    candidates.sort(key=lambda item: int(item["effective_score"]), reverse=True)
    candidates = apply_bridge_fallback_priority(candidates, bridge_fallback_subagent)
    selected = candidates[0]
    eligible_subagents = {item["subagent"] for item in candidates}
    candidate_by_subagent = {item["subagent"]: item for item in candidates}
    requested_fanout = [subagent for subagent in fanout_order if subagent in eligible_subagents]
    default_fanout_min = min(2, len(requested_fanout)) if requested_fanout else 0
    fanout_min_results = max(
        0,
        min(
            policy_int(task_route_cfg.get("fanout_min_results"), default_fanout_min),
            len(requested_fanout),
        ),
    )
    proven_fanout = [
        subagent
        for subagent in requested_fanout
        if int(candidate_by_subagent.get(subagent, {}).get("success_count", 0)) > 0
    ]
    if fanout_min_results > 0 and len(proven_fanout) >= fanout_min_results:
        fanout_subagents = proven_fanout
    else:
        fanout_subagents = requested_fanout
    return {
        "task_class": task_class,
        "selected_subagent": selected["subagent"],
        "reason": f"state={selected['state']}",
        "effective_score": selected["effective_score"],
        "task_fit_score": selected["task_fit_score"],
        "global_score": selected["global_score"],
        "task_class_fit_bonus": selected["task_class_fit_bonus"],
        "task_class_fit_reasons": selected["task_class_fit_reasons"],
        "selected_model": selected["selected_model"],
        "selected_model_source": selected["selected_model_source"],
        "selected_profile": selected["selected_profile"],
        "selected_profile_source": selected["selected_profile_source"],
        "subagent_backend_class": selected["subagent_backend_class"],
        "subagent_state": selected["subagent_state"],
        "capability_band": selected["capability_band"],
        "subagent_write_scope": selected["subagent_write_scope"],
        "orchestration_tier": selected["orchestration_tier"],
        "cost_priority": selected["cost_priority"],
        "write_scope": write_scope,
        "verification_gate": verification_gate,
        "risk_class": risk_class,
        "max_runtime_seconds": selected["max_runtime_seconds"],
        "min_output_bytes": min_output_bytes,
        "fanout_subagents": fanout_subagents,
        "fanout_min_results": fanout_min_results,
        "merge_policy": merge_policy,
        "dispatch_required": dispatch_required,
        "external_first_required": external_first_required,
        "bridge_fallback_subagent": bridge_fallback_subagent,
        "internal_escalation_trigger": internal_escalation_trigger,
        "max_parallel_agents": max_parallel_agents,
        "state_owner": state_owner,
        "suppressed_subagents": suppressed_subagents,
        "fallback_subagents": [
            {
                "subagent": item["subagent"],
                "effective_score": item["effective_score"],
                "task_fit_score": item["task_fit_score"],
                "global_score": item["global_score"],
                "task_class_fit_bonus": item["task_class_fit_bonus"],
                "task_class_fit_reasons": item["task_class_fit_reasons"],
                "selected_model": item["selected_model"],
                "selected_model_source": item["selected_model_source"],
                "selected_profile": item["selected_profile"],
                "selected_profile_source": item["selected_profile_source"],
                "max_runtime_seconds": item["max_runtime_seconds"],
                "subagent_state": item["subagent_state"],
                "orchestration_tier": item["orchestration_tier"],
                "cost_priority": item["cost_priority"],
            }
            for item in candidates[1:]
        ],
    }


def update_score(
    subagent: str,
    result: str,
    task_class: str,
    quality_score: int,
    latency_ms: int,
    note: str,
    domain_tags: list[str] | None = None,
    metrics: dict[str, Any] | None = None,
) -> dict[str, Any]:
    snapshot = runtime_snapshot()
    config = vida_config.load_validated_config()
    scoring_cfg = thresholds(config)
    scorecards = load_json(SCORECARD_PATH, {"subagents": {}})
    subagent_cards = scorecards.setdefault("subagents", {})
    card = subagent_cards.setdefault(subagent, score_defaults())
    global_card = card.setdefault("global", score_defaults()["global"])
    task_card = card.setdefault("by_task_class", {}).setdefault(task_class, dict(global_card))
    domain_buckets = card.setdefault("by_domain", {})
    normalized_domain_tags = [tag for tag in (domain_tags or []) if tag]
    domain_cards = [domain_buckets.setdefault(tag, dict(global_card)) for tag in normalized_domain_tags]
    metrics = metrics or {}
    useful_progress = bool(metrics.get("useful_progress", False))
    timeout_after_progress = bool(metrics.get("timeout_after_progress", False))
    chatter_only = bool(metrics.get("chatter_only", False))
    time_to_first_useful_output_ms = metrics.get("time_to_first_useful_output_ms")

    if result == "success":
        delta = 8 + max(0, min(10, (quality_score - 70) // 5))
        for bucket in (global_card, task_card, *domain_cards):
            bucket["success_count"] = int(bucket.get("success_count", 0)) + 1
            bucket["consecutive_failures"] = 0
    else:
        delta = -15
        for bucket in (global_card, task_card, *domain_cards):
            bucket["failure_count"] = int(bucket.get("failure_count", 0)) + 1
            bucket["consecutive_failures"] = int(bucket.get("consecutive_failures", 0)) + 1

    for bucket in (global_card, task_card, *domain_cards):
        next_score = max(0, min(100, int(bucket.get("score", 50)) + delta))
        bucket["score"] = next_score
        bucket["state"] = classify_state(
            next_score,
            int(bucket.get("consecutive_failures", 0)),
            scoring_cfg,
        )
        bucket["last_result"] = result
        bucket["last_quality_score"] = quality_score
        bucket["last_latency_ms"] = latency_ms
        bucket["last_note"] = note
        if useful_progress:
            bucket["useful_progress_count"] = int(bucket.get("useful_progress_count", 0)) + 1
        if chatter_only:
            bucket["chatter_only_count"] = int(bucket.get("chatter_only_count", 0)) + 1
        if timeout_after_progress:
            bucket["timeout_after_progress_count"] = int(bucket.get("timeout_after_progress_count", 0)) + 1
        if isinstance(time_to_first_useful_output_ms, int) and time_to_first_useful_output_ms > 0:
            avg, samples = update_average(
                int(bucket.get("avg_time_to_first_useful_output_ms", 0)),
                int(bucket.get("time_to_first_useful_output_samples", 0)),
                time_to_first_useful_output_ms,
            )
            bucket["avg_time_to_first_useful_output_ms"] = avg
            bucket["time_to_first_useful_output_samples"] = samples
        total_runs = int(bucket.get("success_count", 0)) + int(bucket.get("failure_count", 0))
        bucket["useful_progress_rate"] = round(
            int(bucket.get("useful_progress_count", 0)) / total_runs,
            3,
        ) if total_runs > 0 else 0
        apply_availability_metrics(bucket, result, metrics)
        bucket["updated_at"] = now_utc()

    save_json(SCORECARD_PATH, scorecards)
    snapshot["scorecards"] = scorecards["subagents"]
    snapshot["written_at"] = now_utc()
    save_json(INIT_PATH, snapshot)
    return {
        "subagent": subagent,
        "task_class": task_class,
        "result": result,
        "global": global_card,
        "task_class_card": task_card,
        "domain_cards": {tag: domain_buckets.get(tag, {}) for tag in normalized_domain_tags},
    }


def usage() -> None:
    print(
        "Usage:\n"
        "  python3 _vida/scripts/subagent-system.py init [task_id]\n"
        "  python3 _vida/scripts/subagent-system.py status\n"
        "  python3 _vida/scripts/subagent-system.py subagents\n"
        "  python3 _vida/scripts/subagent-system.py route <task_class>\n"
        "  python3 _vida/scripts/subagent-system.py probe <subagent>\n"
        "  python3 _vida/scripts/subagent-system.py recover <subagent>\n"
        "  python3 _vida/scripts/subagent-system.py recover-pending\n"
        "  python3 _vida/scripts/subagent-system.py record <subagent> <success|failure> <task_class> [quality_score] [latency_ms] [note]\n"
        "  python3 _vida/scripts/subagent-system.py scorecard [subagent]",
        file=sys.stderr,
    )


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        usage()
        return 1
    cmd = argv[1]
    try:
        if cmd == "init":
            task_id = argv[2] if len(argv) > 2 else None
            print(json.dumps(init_snapshot(task_id), indent=2, sort_keys=True))
            return 0
        if cmd == "status":
            payload = runtime_snapshot()
            print(json.dumps(payload, indent=2, sort_keys=True))
            return 0
        if cmd == "subagents":
            print(json.dumps(subagent_operator_status(), indent=2, sort_keys=True))
            return 0
        if cmd == "route":
            if len(argv) < 3:
                usage()
                return 1
            print(json.dumps(route_subagent(argv[2]), indent=2, sort_keys=True))
            return 0
        if cmd == "probe":
            if len(argv) < 3:
                usage()
                return 1
            print(json.dumps(probe_subagent(argv[2]), indent=2, sort_keys=True))
            return 0
        if cmd == "recover":
            if len(argv) < 3:
                usage()
                return 1
            print(json.dumps(recover_subagent(argv[2]), indent=2, sort_keys=True))
            return 0
        if cmd == "recover-pending":
            print(json.dumps(recover_pending_subagents(), indent=2, sort_keys=True))
            return 0
        if cmd == "record":
            if len(argv) < 5:
                usage()
                return 1
            subagent = argv[2]
            result = argv[3]
            task_class = argv[4]
            quality_score = int(argv[5]) if len(argv) > 5 else (85 if result == "success" else 20)
            latency_ms = int(argv[6]) if len(argv) > 6 else 0
            note = argv[7] if len(argv) > 7 else ""
            print(json.dumps(update_score(subagent, result, task_class, quality_score, latency_ms, note), indent=2, sort_keys=True))
            return 0
    except (ValueError, vida_config.OverlayValidationError) as exc:
        print(f"[subagent-system] {exc}", file=sys.stderr)
        return 2
    if cmd == "scorecard":
        payload = load_json(SCORECARD_PATH, {"subagents": {}})
        if len(argv) > 2:
            print(json.dumps(payload.get("subagents", {}).get(argv[2], {}), indent=2, sort_keys=True))
        else:
            print(json.dumps(payload, indent=2, sort_keys=True))
        return 0
    usage()
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
