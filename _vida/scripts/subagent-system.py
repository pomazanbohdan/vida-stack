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
LEASE_PATH = STATE_DIR / "subagent-leases.json"

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
    bucket.pop("provider_state", None)
    last_note = bucket.get("last_note")
    if isinstance(last_note, str) and "provider_state=" in last_note:
        bucket["last_note"] = last_note.replace("provider_state=", "subagent_state=")


def migrate_domain_buckets(domain_buckets: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(domain_buckets, dict):
        return {}
    migrated: dict[str, Any] = {}
    for raw_key, bucket in domain_buckets.items():
        normalized_key = normalize_domain_tag(raw_key)
        if not normalized_key:
            continue
        current = migrated.setdefault(normalized_key, {})
        if isinstance(bucket, dict):
            current.update(bucket)
            migrate_scorecard_bucket(current)
    return migrated


def save_json(path: Path, payload: Any) -> None:
    ensure_state_dir()
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def load_strategy_memory() -> dict[str, Any]:
    payload = load_json(STRATEGY_PATH, {})
    return payload if isinstance(payload, dict) else {}


DOMAIN_TAG_ALIASES = {
    "odoo_api": "api_contract",
    "flutter_ui": "frontend_ui",
    "riverpod_state": "state_management",
}


def normalize_domain_tag(tag: Any) -> str:
    text = policy_value(tag, "").casefold()
    return DOMAIN_TAG_ALIASES.get(text, text)


def canonicalize_note_text(value: Any) -> Any:
    if not isinstance(value, str):
        return value
    out = value
    replacements = {
        "provider_state=": "subagent_state=",
        "odoo_api": "api_contract",
        "flutter_ui": "frontend_ui",
        "riverpod_state": "state_management",
    }
    for source, target in replacements.items():
        out = out.replace(source, target)
    return out


def normalize_domain_tags(tags: list[Any] | None) -> list[str]:
    out: list[str] = []
    for tag in tags or []:
        normalized = normalize_domain_tag(tag)
        if normalized and normalized not in out:
            out.append(normalized)
    return out


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
    sanitize_runtime_payload(snapshot)
    return snapshot


def sanitize_runtime_payload(payload: Any) -> Any:
    if isinstance(payload, dict):
        for key, value in list(payload.items()):
            if key == "last_note":
                payload[key] = canonicalize_note_text(value)
            else:
                payload[key] = sanitize_runtime_payload(value)
        return payload
    if isinstance(payload, list):
        for idx, value in enumerate(payload):
            payload[idx] = sanitize_runtime_payload(value)
        return payload
    return payload


def runtime_snapshot(task_id: str | None = None) -> dict[str, Any]:
    snapshot = migrate_legacy_runtime_state(load_json(INIT_PATH, {}))
    if snapshot.get("subagents"):
        config = vida_config.load_validated_config()
        current_subagents = detect_subagents(config)
        scoring_cfg = thresholds(config)
        mode, reasons = effective_mode(config, current_subagents)
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
                card["by_domain"] = migrate_domain_buckets(card.get("by_domain", {}) or {})
                for bucket in (card.get("by_domain", {}) or {}).values():
                    migrate_scorecard_bucket(bucket)
            for subagent_name, payload in (scorecards.get("subagents", {}) or {}).items():
                if not isinstance(payload, dict):
                    continue
                global_card = payload.setdefault("global", score_defaults()["global"].copy())
                global_card["lifecycle_stage"] = lifecycle_stage_for(
                    current_subagents.get(subagent_name, {}),
                    global_card,
                    scoring_cfg,
                )
            snapshot["scorecards"] = scorecards.get("subagents", snapshot.get("scorecards", {}))
        snapshot["config_path"] = str(vida_config.CONFIG_PATH) if vida_config.CONFIG_PATH.exists() else ""
        snapshot["protocol_activation"] = {
            "agent_system": bool(vida_config.dotted_get(config, "protocol_activation.agent_system", False)),
        }
        snapshot["agent_system"] = {
            "init_on_boot": bool(vida_config.dotted_get(config, "agent_system.init_on_boot", False)),
            "requested_mode": str(vida_config.dotted_get(config, "agent_system.mode", "native")),
            "effective_mode": mode,
            "state_owner": str(vida_config.dotted_get(config, "agent_system.state_owner", "orchestrator_only")),
            "max_parallel_agents": int(vida_config.dotted_get(config, "agent_system.max_parallel_agents", 1)),
            "scoring": scoring_cfg,
            "reasons": reasons,
        }
        snapshot["subagents"] = current_subagents
        snapshot["task_id"] = task_id or snapshot.get("task_id")
        return snapshot
    return init_snapshot(task_id)


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def load_leases() -> dict[str, Any]:
    payload = load_json(LEASE_PATH, {"leases": {}, "next_fencing_token": 1, "history": []})
    if not isinstance(payload, dict):
        return {"leases": {}, "next_fencing_token": 1, "history": []}
    payload.setdefault("leases", {})
    payload.setdefault("next_fencing_token", 1)
    payload.setdefault("history", [])
    return payload


def save_leases(payload: dict[str, Any]) -> None:
    save_json(LEASE_PATH, payload)


def cleanup_leases(payload: dict[str, Any], *, retain_hours: int = 24) -> dict[str, Any]:
    leases = payload.setdefault("leases", {})
    history = payload.setdefault("history", [])
    now = now_utc_dt()
    retain_after = now - timedelta(hours=max(1, retain_hours))
    pruned: dict[str, Any] = {}
    for key, lease in leases.items():
        if not isinstance(lease, dict):
            continue
        expires_at = parse_utc_timestamp(lease.get("expires_at"))
        released_at = parse_utc_timestamp(lease.get("released_at"))
        if lease.get("status") == "active" and expires_at is not None and expires_at <= now:
            lease["status"] = "expired"
            lease["expired_at"] = now_utc()
        status = policy_value(lease.get("status"), "active")
        keep = True
        if status in {"released", "expired"}:
            marker = released_at or expires_at
            if marker is not None and marker < retain_after:
                keep = False
        if keep:
            pruned[key] = lease
    payload["leases"] = pruned
    payload["history"] = [
        item
        for item in history[-200:]
        if isinstance(item, dict)
    ]
    return payload


def acquire_lease(resource_type: str, resource_id: str, holder: str, ttl_seconds: int = 3600) -> dict[str, Any]:
    payload = cleanup_leases(load_leases())
    leases = payload.setdefault("leases", {})
    history = payload.setdefault("history", [])
    key = f"{resource_type}:{resource_id}"
    now = now_utc_dt()
    current = leases.get(key)
    if isinstance(current, dict):
        expires_at = parse_utc_timestamp(current.get("expires_at"))
        current_status = policy_value(current.get("status"), "active")
        if (
            current_status == "active"
            and current.get("holder") != holder
            and expires_at is not None
            and expires_at > now
        ):
            current["conflict_count"] = int(current.get("conflict_count", 0) or 0) + 1
            current["last_conflict_at"] = now_utc()
            current["last_conflict_holder"] = holder
            history.append(
                {
                    "ts": now_utc(),
                    "resource_type": resource_type,
                    "resource_id": resource_id,
                    "holder": holder,
                    "active_holder": current.get("holder"),
                    "event": "lease_conflict",
                }
            )
            payload["history"] = history[-50:]
            save_leases(payload)
            return {
                "status": "blocked",
                "resource_type": resource_type,
                "resource_id": resource_id,
                "event": "lease_conflict",
                "lease": current,
            }
    fencing_token = int(payload.get("next_fencing_token", 1) or 1)
    payload["next_fencing_token"] = fencing_token + 1
    lease = {
        "resource_type": resource_type,
        "resource_id": resource_id,
        "holder": holder,
        "acquired_at": now_utc(),
        "expires_at": future_utc_iso(minutes=max(1, ttl_seconds // 60)),
        "fencing_token": fencing_token,
        "status": "active",
        "conflict_count": 0,
    }
    leases[key] = lease
    history.append(
        {
            "ts": now_utc(),
            "resource_type": resource_type,
            "resource_id": resource_id,
            "holder": holder,
            "event": "lease_acquired",
            "fencing_token": fencing_token,
        }
    )
    payload["history"] = history[-50:]
    save_leases(payload)
    return {"status": "acquired", "lease": lease}


def renew_lease(resource_type: str, resource_id: str, holder: str, ttl_seconds: int = 3600) -> dict[str, Any]:
    payload = cleanup_leases(load_leases())
    leases = payload.setdefault("leases", {})
    history = payload.setdefault("history", [])
    key = f"{resource_type}:{resource_id}"
    current = leases.get(key)
    if not isinstance(current, dict):
        return {"status": "noop", "reason": "missing"}
    if current.get("holder") != holder:
        current["conflict_count"] = int(current.get("conflict_count", 0) or 0) + 1
        current["last_conflict_at"] = now_utc()
        current["last_conflict_holder"] = holder
        history.append(
            {
                "ts": now_utc(),
                "resource_type": resource_type,
                "resource_id": resource_id,
                "holder": holder,
                "active_holder": current.get("holder"),
                "event": "renew_conflict",
            }
        )
        payload["history"] = history[-200:]
        save_leases(payload)
        return {"status": "blocked", "reason": "holder_mismatch", "lease": current}
    if policy_value(current.get("status"), "active") != "active":
        return {"status": "noop", "reason": "not_active", "lease": current}
    current["renewed_at"] = now_utc()
    current["expires_at"] = future_utc_iso(minutes=max(1, ttl_seconds // 60))
    history.append(
        {
            "ts": now_utc(),
            "resource_type": resource_type,
            "resource_id": resource_id,
            "holder": holder,
            "event": "lease_renewed",
            "fencing_token": current.get("fencing_token"),
        }
    )
    payload["history"] = history[-200:]
    save_leases(payload)
    return {"status": "renewed", "lease": current}


def release_lease(resource_type: str, resource_id: str, holder: str) -> dict[str, Any]:
    payload = cleanup_leases(load_leases())
    leases = payload.setdefault("leases", {})
    history = payload.setdefault("history", [])
    key = f"{resource_type}:{resource_id}"
    current = leases.get(key)
    if not isinstance(current, dict):
        return {"status": "noop", "reason": "missing"}
    if current.get("holder") != holder:
        current["conflict_count"] = int(current.get("conflict_count", 0) or 0) + 1
        current["last_conflict_at"] = now_utc()
        current["last_conflict_holder"] = holder
        history.append(
            {
                "ts": now_utc(),
                "resource_type": resource_type,
                "resource_id": resource_id,
                "holder": holder,
                "active_holder": current.get("holder"),
                "event": "release_conflict",
            }
        )
        payload["history"] = history[-200:]
        save_leases(payload)
        return {"status": "blocked", "reason": "holder_mismatch", "lease": current}
    current["status"] = "released"
    current["released_at"] = now_utc()
    history.append(
        {
            "ts": now_utc(),
            "resource_type": resource_type,
            "resource_id": resource_id,
            "holder": holder,
            "event": "lease_released",
            "fencing_token": current.get("fencing_token"),
        }
    )
    payload["history"] = history[-200:]
    save_leases(payload)
    return {"status": "released", "lease": current}


def active_leases() -> dict[str, Any]:
    payload = cleanup_leases(load_leases())
    leases = payload.setdefault("leases", {})
    history = payload.setdefault("history", [])
    rows: list[dict[str, Any]] = []
    by_resource_type: dict[str, dict[str, int]] = {}
    for key, lease in leases.items():
        if not isinstance(lease, dict):
            continue
        rows.append({"key": key, **lease})
        resource_type = policy_value(lease.get("resource_type"), "unknown")
        status = policy_value(lease.get("status"), "active")
        bucket = by_resource_type.setdefault(resource_type, {"active": 0, "released": 0, "expired": 0})
        bucket[status] = int(bucket.get(status, 0) or 0) + 1
    save_leases(payload)
    rows.sort(key=lambda item: (item.get("status") != "active", str(item.get("key"))))
    history_rows = [item for item in history if isinstance(item, dict)][-20:]
    lease_conflicts = sum(1 for item in history_rows if str(item.get("event", "")).endswith("conflict"))
    recent_conflicts_by_resource: dict[str, int] = {}
    for item in history_rows:
        if not str(item.get("event", "")).endswith("conflict"):
            continue
        resource_key = f"{policy_value(item.get('resource_type'), 'unknown')}:{policy_value(item.get('resource_id'), 'unknown')}"
        recent_conflicts_by_resource[resource_key] = recent_conflicts_by_resource.get(resource_key, 0) + 1
    return {
        "generated_at": now_utc(),
        "leases": rows,
        "history": history_rows,
        "summary": {
            "active": sum(1 for item in rows if item.get("status") == "active"),
            "released": sum(1 for item in rows if item.get("status") == "released"),
            "expired": sum(1 for item in rows if item.get("status") == "expired"),
            "recent_conflicts": lease_conflicts,
            "by_resource_type": by_resource_type,
            "recent_conflicts_by_resource": recent_conflicts_by_resource,
        },
    }


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


def target_review_state_for(risk_class: str) -> str:
    normalized = str(risk_class or "R0").upper()
    if normalized == "R0":
        return "review_passed"
    if normalized == "R1":
        return "policy_gate_required"
    if normalized == "R2":
        return "senior_review_required"
    return "human_gate_required"


def target_manifest_review_state_for(risk_class: str) -> str:
    normalized = str(risk_class or "R0").upper()
    if normalized == "R0":
        return "promotion_ready"
    if normalized == "R1":
        return "policy_gate_required"
    if normalized == "R2":
        return "senior_review_required"
    return "human_gate_required"


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
            "budget_cost_units": policy_int(subagent_cfg.get("budget_cost_units"), 0),
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
        "probation_success_runs": int(scoring.get("probation_success_runs", 3)),
        "probation_task_runs": int(scoring.get("probation_task_runs", 1)),
        "retirement_failure_limit": int(scoring.get("retirement_failure_limit", 12)),
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
            "startup_timeout_count": 0,
            "no_output_timeout_count": 0,
            "stalled_after_progress_count": 0,
            "time_to_first_useful_output_samples": 0,
            "avg_time_to_first_useful_output_ms": 0,
            "useful_progress_rate": 0,
            "subagent_state": "active",
            "failure_reason": "",
            "cooldown_until": "",
            "probe_required": False,
            "last_quota_exhausted_at": "",
            "recovery_attempt_count": 0,
            "recovery_success_count": 0,
            "last_recovery_at": "",
            "last_recovery_status": "",
            "authored_runs_count": 0,
            "authored_verified_pass_count": 0,
            "authored_verified_fail_count": 0,
            "verifier_runs_count": 0,
            "verifier_success_count": 0,
            "verifier_catch_count": 0,
            "lifecycle_stage": "declared",
            "retirement_reason": "",
        },
        "by_task_class": {},
        "by_domain": {},
    }


def lifecycle_stage_for(
    subagent_cfg: dict[str, Any],
    global_card: dict[str, Any],
    scoring_cfg: dict[str, int],
) -> str:
    enabled = bool(subagent_cfg.get("enabled", False))
    available = bool(subagent_cfg.get("available", False))
    subagent_state = policy_value(global_card.get("subagent_state"), "active")
    score_state = policy_value(global_card.get("state"), "normal")
    success_count = int(global_card.get("success_count", 0) or 0)
    failure_count = int(global_card.get("failure_count", 0) or 0)
    last_probe_at = policy_value(global_card.get("last_probe_at"), "")
    last_recovery_status = policy_value(global_card.get("last_recovery_status"), "")
    last_recovery_at = policy_value(global_card.get("last_recovery_at"), "")
    cooldown_until = parse_utc_timestamp(global_card.get("cooldown_until"))
    probation_success_runs = max(1, int(scoring_cfg.get("probation_success_runs", 3)))
    retirement_failure_limit = max(1, int(scoring_cfg.get("retirement_failure_limit", 12)))

    if not enabled or subagent_state == "disabled_manual":
        return "retired"
    if failure_count >= retirement_failure_limit and success_count <= 0:
        return "retired"
    if cooldown_until is not None and cooldown_until > now_utc_dt():
        return "cooldown"
    if subagent_state in {"degraded", "quota_exhausted"} or score_state == "demoted":
        return "degraded"
    if last_recovery_status == "success" and last_recovery_at:
        return "recovered"
    if score_state == "preferred" or success_count >= probation_success_runs:
        return "promoted"
    if last_probe_at:
        return "probation" if success_count > 0 else "probed"
    if available:
        return "detected"
    return "declared"


def lane_lifecycle_stage_for(
    task_card: dict[str, Any],
    global_stage: str,
    scoring_cfg: dict[str, int],
) -> str:
    if not isinstance(task_card, dict) or not task_card:
        return global_stage
    state = policy_value(task_card.get("state"), "normal")
    success_count = int(task_card.get("success_count", 0) or 0)
    score = int(task_card.get("score", 50) or 50)
    probation_task_runs = max(1, int(scoring_cfg.get("probation_task_runs", 1)))
    promotion_score = int(scoring_cfg.get("promotion_score", 80))
    if state == "demoted":
        return "degraded"
    if state == "preferred" or (score >= promotion_score and success_count >= probation_task_runs):
        return "promoted"
    if success_count > 0:
        return "probation"
    return global_stage


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
        bucket["retirement_reason"] = ""
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
    if policy_value(bucket.get("subagent_state"), "") == "disabled_manual":
        bucket["retirement_reason"] = failure_reason or "disabled_manual"
    apply_behavioral_degradation(bucket)
    normalize_availability_bucket(bucket)


def update_subagent_availability(
    subagent: str,
    metrics: dict[str, Any],
    note: str = "",
    *,
    recovery_attempted: bool = False,
    recovery_success: bool = False,
) -> dict[str, Any]:
    config = vida_config.load_validated_config()
    scoring_cfg = thresholds(config)
    snapshot = runtime_snapshot()
    scorecards = load_json(SCORECARD_PATH, {"subagents": {}})
    subagent_cards = scorecards.setdefault("subagents", {})
    card = subagent_cards.setdefault(subagent, score_defaults())
    global_card = card.setdefault("global", score_defaults()["global"])
    apply_availability_metrics(global_card, "success" if metrics.get("subagent_state") == "active" else "failure", metrics)
    global_card["last_probe_note"] = note
    global_card["last_probe_at"] = now_utc()
    if recovery_attempted:
        global_card["recovery_attempt_count"] = int(global_card.get("recovery_attempt_count", 0)) + 1
        global_card["last_recovery_at"] = now_utc()
        global_card["last_recovery_status"] = "success" if recovery_success else "failure"
        if recovery_success:
            global_card["recovery_success_count"] = int(global_card.get("recovery_success_count", 0)) + 1
    global_card["lifecycle_stage"] = lifecycle_stage_for(snapshot.get("subagents", {}).get(subagent, {}), global_card, scoring_cfg)
    save_json(SCORECARD_PATH, scorecards)
    snapshot["scorecards"] = scorecards["subagents"]
    snapshot["written_at"] = now_utc()
    save_json(INIT_PATH, snapshot)
    return {"subagent": subagent, "availability": global_card}


def subagent_operator_status() -> dict[str, Any]:
    config = vida_config.load_validated_config()
    scoring_cfg = thresholds(config)
    snapshot = runtime_snapshot()
    subagents = snapshot.get("subagents", {})
    scorecards = snapshot.get("scorecards", {})
    lease_status = active_leases()
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
        lifecycle_stage = lifecycle_stage_for(subagent_cfg, global_card, scoring_cfg)
        recovery_attempt_count = int(global_card.get("recovery_attempt_count", 0) or 0)
        recovery_success_count = int(global_card.get("recovery_success_count", 0) or 0)
        startup_timeout_count = int(global_card.get("startup_timeout_count", 0) or 0)
        no_output_timeout_count = int(global_card.get("no_output_timeout_count", 0) or 0)
        stalled_after_progress_count = int(global_card.get("stalled_after_progress_count", 0) or 0)
        task_class_cards = subagent_scorecard.get("by_task_class", {}) or {}
        preferred_task_classes = sorted(
            [
                task_class
                for task_class, bucket in task_class_cards.items()
                if isinstance(bucket, dict) and policy_value(bucket.get("state"), "normal") == "preferred"
            ]
        )
        eligible_task_classes = sorted(
            [
                task_class
                for task_class, bucket in task_class_cards.items()
                if isinstance(bucket, dict)
                and policy_value(bucket.get("state"), "normal") != "demoted"
                and int(bucket.get("score", 50) or 50) >= 60
            ]
        )
        row = {
            "subagent": subagent_name,
            "available": bool(subagent_cfg.get("available", False)),
            "lifecycle_stage": lifecycle_stage,
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
            "recovery_attempt_count": recovery_attempt_count,
            "recovery_success_count": recovery_success_count,
            "last_recovery_status": policy_value(global_card.get("last_recovery_status"), ""),
            "last_recovery_at": policy_value(global_card.get("last_recovery_at"), ""),
            "authored_runs_count": int(global_card.get("authored_runs_count", 0) or 0),
            "authored_verified_pass_count": int(global_card.get("authored_verified_pass_count", 0) or 0),
            "authored_verified_fail_count": int(global_card.get("authored_verified_fail_count", 0) or 0),
            "verifier_runs_count": int(global_card.get("verifier_runs_count", 0) or 0),
            "verifier_success_count": int(global_card.get("verifier_success_count", 0) or 0),
            "verifier_catch_count": int(global_card.get("verifier_catch_count", 0) or 0),
            "startup_timeout_count": startup_timeout_count,
            "no_output_timeout_count": no_output_timeout_count,
            "stalled_after_progress_count": stalled_after_progress_count,
            "quality_tier": policy_value(subagent_cfg.get("quality_tier"), "unknown"),
            "billing_tier": policy_value(subagent_cfg.get("billing_tier"), "unknown"),
            "budget_cost_units": policy_int(subagent_cfg.get("budget_cost_units"), 0),
            "preferred_task_classes": preferred_task_classes,
            "eligible_task_classes": eligible_task_classes,
            "lane_stages": {
                task_class: lane_lifecycle_stage_for(bucket, lifecycle_stage, scoring_cfg)
                for task_class, bucket in sorted(task_class_cards.items())
                if isinstance(bucket, dict)
            },
            "recommended_action": (
                "retired_manual"
                if lifecycle_stage == "retired"
                else
                "investigate"
                if lifecycle_stage == "degraded"
                else
                "build_lane_history"
                if lifecycle_stage == "probation"
                else
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
    recent_recoveries = [
        {
            "subagent": row["subagent"],
            "last_recovery_at": row["last_recovery_at"],
            "last_recovery_status": row["last_recovery_status"],
            "recovery_attempt_count": row["recovery_attempt_count"],
            "recovery_success_count": row["recovery_success_count"],
        }
        for row in rows
        if row["last_recovery_at"]
    ]
    recent_recoveries.sort(key=lambda item: item["last_recovery_at"], reverse=True)
    unstable_by_timeout_class = [
        {
            "subagent": row["subagent"],
            "startup_timeout_count": row["startup_timeout_count"],
            "no_output_timeout_count": row["no_output_timeout_count"],
            "stalled_after_progress_count": row["stalled_after_progress_count"],
        }
        for row in rows
        if row["startup_timeout_count"] or row["no_output_timeout_count"] or row["stalled_after_progress_count"]
    ]
    routing_cfg = vida_config.dotted_get(config, "agent_system.routing", {}) or {}
    review_targets: dict[str, Any] = {}
    for task_class in sorted(routing_cfg.keys()):
        route = route_subagent(task_class)
        review_targets[task_class] = {
            "risk_class": route.get("risk_class", "R0"),
            "target_review_state": route.get("target_review_state", target_review_state_for("R0")),
            "target_manifest_review_state": route.get(
                "target_manifest_review_state",
                target_manifest_review_state_for("R0"),
            ),
            "verification_gate": route.get("verification_gate", "subagent_return_contract"),
            "write_scope": route.get("write_scope", "none"),
            "independent_verification_required": route.get("independent_verification_required", "no"),
            "verification_route_task_class": route.get("verification_route_task_class", ""),
            "verification_plan": route.get("verification_plan", {}),
            "route_budget": route.get("route_budget", {}),
            "route_graph": route.get("route_graph", {}),
        }
    return {
        "generated_at": now_utc(),
        "subagents": rows,
        "recent_recoveries": recent_recoveries[:5],
        "unstable_by_timeout_class": unstable_by_timeout_class,
        "review_targets": review_targets,
        "leases": lease_status.get("summary", {}),
        "summary": {
            "probation": sum(1 for row in rows if row["lifecycle_stage"] == "probation"),
            "promoted": sum(1 for row in rows if row["lifecycle_stage"] == "promoted"),
            "retired": sum(1 for row in rows if row["lifecycle_stage"] == "retired"),
            "healthy": sum(
                1
                for row in rows
                if row["subagent_state"] == "active"
                and row["lifecycle_stage"] not in {"degraded", "retired"}
                and not row["probe_required"]
                and not row["cooldown_until"]
            ),
            "cooldown": sum(1 for row in rows if row["cooldown_until"]),
            "probe_required": sum(1 for row in rows if row["probe_required"]),
            "degraded": sum(
                1
                for row in rows
                if row["subagent_state"] == "degraded" or row["lifecycle_stage"] == "degraded"
            ),
            "quota_exhausted": sum(1 for row in rows if row["subagent_state"] == "quota_exhausted"),
            "preferred": sum(1 for row in rows if row["state"] == "preferred"),
            "demoted": sum(1 for row in rows if row["state"] == "demoted"),
            "recent_lease_conflicts": int(lease_status.get("summary", {}).get("recent_conflicts", 0) or 0),
        },
    }


def subagent_diagnosis(task_class: str | None = None) -> dict[str, Any]:
    status = subagent_operator_status()
    rows = status.get("subagents", [])
    alerts: list[dict[str, Any]] = []
    recommended_actions: list[dict[str, Any]] = []
    for row in rows:
        subagent = policy_value(row.get("subagent"), "")
        lifecycle_stage = policy_value(row.get("lifecycle_stage"), "")
        recommended_action = policy_value(row.get("recommended_action"), "")
        if lifecycle_stage in {"degraded", "cooldown", "retired"} or row.get("probe_required"):
            alerts.append(
                {
                    "subagent": subagent,
                    "severity": "warn" if lifecycle_stage != "retired" else "info",
                    "kind": "availability_or_lifecycle",
                    "lifecycle_stage": lifecycle_stage,
                    "failure_reason": policy_value(row.get("failure_reason"), ""),
                    "recommended_action": recommended_action,
                }
            )
        lane_stages = row.get("lane_stages", {})
        if isinstance(lane_stages, dict):
            risky_lanes = sorted(
                lane
                for lane, stage in lane_stages.items()
                if stage in {"probation", "degraded"}
            )
            if risky_lanes:
                alerts.append(
                    {
                        "subagent": subagent,
                        "severity": "info",
                        "kind": "lane_readiness",
                        "risky_lanes": risky_lanes,
                        "recommended_action": recommended_action or "build_lane_history",
                    }
                )
        if recommended_action not in {"", "healthy", "none"}:
            recommended_actions.append(
                {
                    "subagent": subagent,
                    "recommended_action": recommended_action,
                    "lifecycle_stage": lifecycle_stage,
                }
            )

    unstable = sorted(
        status.get("unstable_by_timeout_class", []),
        key=lambda item: -(
            int(item.get("startup_timeout_count", 0) or 0)
            + int(item.get("no_output_timeout_count", 0) or 0)
            + int(item.get("stalled_after_progress_count", 0) or 0)
        ),
    )
    lease_summary = status.get("leases", {}) or {}
    diagnosis = {
        "generated_at": now_utc(),
        "summary": status.get("summary", {}),
        "alerts": alerts,
        "recommended_actions": recommended_actions[:8],
        "unstable_by_timeout_class": unstable[:5],
        "recent_recoveries": status.get("recent_recoveries", []),
        "leases": lease_summary,
        "review_targets": status.get("review_targets", {}),
    }
    if task_class:
        diagnosis["task_class"] = task_class
        diagnosis["route"] = route_subagent(task_class)
    return diagnosis


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
        result = update_subagent_availability(
            subagent,
            metrics,
            note,
            recovery_attempted=True,
            recovery_success=success,
        )
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
        result = update_subagent_availability(
            subagent,
            metrics,
            f"probe timeout after {timeout_seconds}s",
            recovery_attempted=True,
            recovery_success=False,
        )
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
    scoring_cfg = thresholds(vida_config.load_validated_config())
    for name in subagents:
        subagent_payload.setdefault(name, score_defaults())
        if isinstance(subagent_payload.get(name), dict):
            global_card = subagent_payload[name].setdefault("global", score_defaults()["global"].copy())
            global_card.setdefault("lifecycle_stage", "declared")
            global_card.setdefault("retirement_reason", "")
    for card in subagent_payload.values():
        if not isinstance(card, dict):
            continue
        migrate_scorecard_bucket(card.get("global", {}))
        for bucket in (card.get("by_task_class", {}) or {}).values():
            migrate_scorecard_bucket(bucket)
        card["by_domain"] = migrate_domain_buckets(card.get("by_domain", {}) or {})
        for bucket in (card.get("by_domain", {}) or {}).values():
            migrate_scorecard_bucket(bucket)
    for subagent_name, card in subagent_payload.items():
        if not isinstance(card, dict):
            continue
        global_card = card.setdefault("global", score_defaults()["global"].copy())
        global_stage = lifecycle_stage_for(subagents.get(subagent_name, {}), global_card, scoring_cfg)
        global_card["lifecycle_stage"] = global_stage
        for task_card in (card.get("by_task_class", {}) or {}).values():
            if isinstance(task_card, dict):
                task_card["lifecycle_stage"] = lane_lifecycle_stage_for(task_card, global_stage, scoring_cfg)
    sanitize_runtime_payload(payload)
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
    for subagent_name, payload in scorecards.get("subagents", {}).items():
        if not isinstance(payload, dict):
            continue
        global_card = payload.setdefault("global", score_defaults()["global"].copy())
        global_card["lifecycle_stage"] = lifecycle_stage_for(subagent_state.get(subagent_name, {}), global_card, scoring_cfg)
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


def verification_task_class_for(task_class: str, write_scope: str) -> str:
    normalized_task = policy_value(task_class, "default")
    normalized_scope = policy_value(write_scope, "none")
    if normalized_task in {"review", "review_ensemble", "verification", "verification_ensemble"}:
        return ""
    if normalized_scope != "none":
        return "review_ensemble"
    mapping = {
        "research": "verification_ensemble",
        "research_fast": "verification_ensemble",
        "ui_research": "verification_ensemble",
        "research_deep": "verification_ensemble",
        "analysis": "verification_ensemble",
        "meta_analysis": "verification_ensemble",
        "architecture": "review_ensemble",
        "small_patch": "review_ensemble",
        "small_patch_write": "review_ensemble",
        "ui_patch": "review_ensemble",
        "implementation": "review_ensemble",
    }
    return mapping.get(normalized_task, "")


def independent_verification_required_for(task_class: str, write_scope: str, dispatch_required: str) -> bool:
    normalized_task = policy_value(task_class, "default")
    normalized_scope = policy_value(write_scope, "none")
    normalized_dispatch = policy_value(dispatch_required, "")
    if normalized_task in {"review", "review_ensemble", "verification", "verification_ensemble"}:
        return False
    if normalized_scope != "none":
        return True
    return normalized_task in {
        "research",
        "research_fast",
        "ui_research",
        "research_deep",
        "analysis",
        "meta_analysis",
        "architecture",
    } or "fanout" in normalized_dispatch or "external_first" in normalized_dispatch


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
    startup_timeout_count = int(
        task_card.get("startup_timeout_count", global_card.get("startup_timeout_count", 0)) or 0
    )
    no_output_timeout_count = int(
        task_card.get("no_output_timeout_count", global_card.get("no_output_timeout_count", 0)) or 0
    )
    stalled_after_progress_count = int(
        task_card.get("stalled_after_progress_count", global_card.get("stalled_after_progress_count", 0)) or 0
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
    timeout_instability = startup_timeout_count + no_output_timeout_count + stalled_after_progress_count
    if timeout_instability > 0:
        budget -= min(15, timeout_instability * 4)
    if speed_tier == "fast":
        budget = min(budget, baseline + 20)

    return max(120, min(300, budget))


def recovery_routing_adjustment(global_card: dict[str, Any], task_card: dict[str, Any]) -> tuple[int, list[str], bool]:
    recovery_attempts = int(global_card.get("recovery_attempt_count", 0) or 0)
    recovery_successes = int(global_card.get("recovery_success_count", 0) or 0)
    if recovery_attempts <= 0:
        return 0, [], False
    success_ratio = recovery_successes / max(1, recovery_attempts)
    last_recovery_status = policy_value(global_card.get("last_recovery_status"), "")
    reasons: list[str] = []
    bonus = 0
    recovered_recently = last_recovery_status == "success" and success_ratio >= 0.5
    if recovered_recently:
        bonus += 6
        reasons.append("recovery:recent_success")
    elif last_recovery_status == "failure":
        bonus -= min(12, recovery_attempts * 2)
        reasons.append("recovery:failed")
    if recovery_attempts >= 3 and success_ratio < 0.34:
        bonus -= 6
        reasons.append("recovery:unstable")
    return bonus, reasons, recovered_recently


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


def strategy_memory_adjustment(task_class: str, subagent: str, strategy: dict[str, Any]) -> tuple[int, list[str]]:
    task_entry = (strategy.get("task_classes", {}) or {}).get(task_class, {})
    memory_hints = task_entry.get("memory_hints", {}) if isinstance(task_entry, dict) else {}
    if not isinstance(memory_hints, dict):
        return 0, []
    bonus = 0
    reasons: list[str] = []
    preferred = set(split_csv(memory_hints.get("preferred_subagents")))
    avoid = set(split_csv(memory_hints.get("avoid_subagents")))
    retry_useful = set(split_csv(memory_hints.get("retry_useful_subagents")))
    failure_prone = set(split_csv(memory_hints.get("failure_prone_subagents")))
    if subagent in preferred:
        bonus += 8
        reasons.append("memory:preferred")
    if subagent in retry_useful:
        bonus += 4
        reasons.append("memory:retry_useful")
    if subagent in avoid:
        bonus -= 8
        reasons.append("memory:avoid")
    if subagent in failure_prone:
        bonus -= 6
        reasons.append("memory:failure_prone")
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


def subagent_budget_cost_units(payload: dict[str, Any]) -> int:
    explicit = payload.get("budget_cost_units")
    if isinstance(explicit, int) and explicit >= 0:
        return explicit
    billing_tier = policy_value(payload.get("billing_tier"), "")
    orchestration_tier = policy_value(payload.get("orchestration_tier"), "")
    if billing_tier == "free" or orchestration_tier == "external_free":
        return 0
    if orchestration_tier == "bridge" or billing_tier == "low":
        return 3
    if orchestration_tier == "senior" or billing_tier == "internal":
        return 8
    return 2


def route_budget_limits(task_route_cfg: dict[str, Any], write_scope: str, dispatch_required: str) -> dict[str, int | str]:
    max_budget_units = max(0, policy_int(task_route_cfg.get("max_budget_units"), 0))
    if max_budget_units <= 0:
        max_budget_units = 2 if write_scope == "none" else 6
        if "fanout" in dispatch_required:
            max_budget_units += 2
    max_cli_subagent_calls = max(1, policy_int(task_route_cfg.get("max_cli_subagent_calls"), 0))
    if max_cli_subagent_calls <= 1:
        max_cli_subagent_calls = 5 if "fanout" in dispatch_required else 3
    max_verification_passes = max(0, policy_int(task_route_cfg.get("max_verification_passes"), 1))
    max_fallback_hops = max(0, policy_int(task_route_cfg.get("max_fallback_hops"), 1))
    max_total_runtime_seconds = max(30, policy_int(task_route_cfg.get("max_total_runtime_seconds"), 0))
    if max_total_runtime_seconds <= 30:
        max_total_runtime_seconds = max(180, policy_int(task_route_cfg.get("max_runtime_seconds"), 180))
        if "fanout" in dispatch_required:
            max_total_runtime_seconds *= 2
    return {
        "budget_policy": policy_value(task_route_cfg.get("budget_policy"), "balanced"),
        "max_budget_units": max_budget_units,
        "max_cli_subagent_calls": max_cli_subagent_calls,
        "max_verification_passes": max_verification_passes,
        "max_fallback_hops": max_fallback_hops,
        "max_total_runtime_seconds": max_total_runtime_seconds,
    }


def budget_efficiency_adjustment(
    payload: dict[str, Any],
    task_card: dict[str, Any],
    global_card: dict[str, Any],
    budget_policy: str,
    max_budget_units: int,
) -> tuple[int, list[str]]:
    reasons: list[str] = []
    cost_units = subagent_budget_cost_units(payload)
    useful_progress_rate = float(task_card.get("useful_progress_rate", global_card.get("useful_progress_rate", 0)) or 0)
    avg_ttfu_ms = policy_int(
        task_card.get("avg_time_to_first_useful_output_ms"),
        policy_int(global_card.get("avg_time_to_first_useful_output_ms"), 0),
    )
    adjustment = 0
    if budget_policy in {"balanced", "strict"}:
        adjustment -= min(18, cost_units * (4 if budget_policy == "strict" else 3))
        if cost_units == 0:
            adjustment += 6
            reasons.append("budget:free_lane")
    if max_budget_units > 0 and cost_units > max_budget_units:
        adjustment -= min(25, (cost_units - max_budget_units) * 6)
        reasons.append("budget:over_cap")
    elif max_budget_units > 0 and cost_units >= max(1, max_budget_units // 2):
        adjustment -= 4
        reasons.append("budget:near_cap")
    if useful_progress_rate >= 0.7 and cost_units <= max(1, max_budget_units):
        adjustment += 6
        reasons.append("budget:efficient_progress")
    elif useful_progress_rate >= 0.4 and cost_units == 0:
        adjustment += 3
        reasons.append("budget:free_progress")
    if avg_ttfu_ms > 0 and avg_ttfu_ms <= 120000 and cost_units <= max(1, max_budget_units):
        adjustment += 3
        reasons.append("budget:fast_first_signal")
    elif avg_ttfu_ms > 240000 and cost_units > 0:
        adjustment -= 4
        reasons.append("budget:slow_paid_signal")
    return adjustment, reasons


def build_route_graph(
    *,
    task_class: str,
    dispatch_required: str,
    deterministic_first: str,
    graph_strategy: str,
    selected_subagent: str,
    fanout_subagents: list[str],
    verification_plan: dict[str, Any],
    bridge_fallback_subagent: str,
    internal_escalation_trigger: str,
    budget: dict[str, Any],
) -> dict[str, Any]:
    primary_mode = "fanout" if fanout_subagents else "single"
    nodes: list[dict[str, Any]] = [
        {"id": "entry", "type": "entry", "label": task_class},
        {
            "id": "primary",
            "type": "dispatch",
            "mode": primary_mode,
            "selected_subagent": selected_subagent,
            "fanout_subagents": fanout_subagents,
        },
    ]
    edges: list[dict[str, Any]] = [
        {"from": "entry", "to": "primary", "condition": "route_selected"},
    ]
    path = ["entry", "primary"]
    if bridge_fallback_subagent:
        nodes.append(
            {
                "id": "bridge_fallback",
                "type": "fallback",
                "selected_subagent": bridge_fallback_subagent,
                "condition": "primary_route_insufficient",
            }
        )
        edges.append(
            {
                "from": "primary",
                "to": "bridge_fallback",
                "condition": "subagent_exhausted_or_quality_gap",
            }
        )
    if internal_escalation_trigger:
        nodes.append(
            {
                "id": "internal_escalation",
                "type": "escalation",
                "selected_subagent": "internal_subagents",
                "condition": internal_escalation_trigger,
            }
        )
        edges.append(
            {
                "from": "bridge_fallback" if bridge_fallback_subagent else "primary",
                "to": "internal_escalation",
                "condition": internal_escalation_trigger,
            }
        )
    if verification_plan.get("required") == "yes":
        nodes.append(
            {
                "id": "verification",
                "type": "verification",
                "route_task_class": verification_plan.get("route_task_class"),
                "selected_subagent": verification_plan.get("selected_subagent"),
                "independent": bool(verification_plan.get("independent", False)),
            }
        )
        edges.append(
            {
                "from": "primary",
                "to": "verification",
                "condition": "primary_result_ready",
            }
        )
        path.append("verification")
    nodes.append(
        {
            "id": "synthesis",
            "type": "synthesis",
            "owner": "orchestrator",
            "budget_policy": budget.get("budget_policy"),
        }
    )
    edges.append(
        {
            "from": "verification" if verification_plan.get("required") == "yes" else "primary",
            "to": "synthesis",
            "condition": "verification_gate_passed_or_not_required",
        }
    )
    path.append("synthesis")
    return {
        "graph_strategy": graph_strategy,
        "deterministic_first": deterministic_first,
        "primary_mode": primary_mode,
        "nodes": nodes,
        "edges": edges,
        "planned_path": path,
    }


def build_route_budget(
    *,
    budget_limits: dict[str, Any],
    selected: dict[str, Any],
    fanout_subagents: list[str],
    verification_plan: dict[str, Any],
    bridge_fallback_subagent: str,
) -> dict[str, Any]:
    primary_calls = len(fanout_subagents) if fanout_subagents else 1
    verification_calls = 1 if verification_plan.get("required") == "yes" and verification_plan.get("selected_subagent") else 0
    fallback_hops = 1 if bridge_fallback_subagent else 0
    selected_cost_units = int(selected.get("budget_cost_units", 0))
    estimated_units = selected_cost_units
    if fanout_subagents:
        estimated_units = 0
    if verification_calls and verification_plan.get("selected_subagent") == bridge_fallback_subagent:
        estimated_units += 3
    return {
        **budget_limits,
        "estimated_primary_calls": primary_calls,
        "estimated_verification_calls": verification_calls,
        "estimated_fallback_hops": fallback_hops,
        "estimated_selected_cost_units": selected_cost_units,
        "estimated_route_cost_units": estimated_units,
    }


def route_candidate_context(
    task_class: str,
    snapshot: dict[str, Any],
    config: dict[str, Any],
    strategy: dict[str, Any],
    *,
    excluded_subagents: set[str] | None = None,
) -> dict[str, Any]:
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
    graph_strategy = policy_value(task_route_cfg.get("graph_strategy"), "deterministic_then_escalate")
    deterministic_first = policy_value(task_route_cfg.get("deterministic_first"), "yes" if external_first_required == "yes" else "no")
    budget_limits = route_budget_limits(task_route_cfg, write_scope, dispatch_required)
    verification_route_task_class = policy_value(
        task_route_cfg.get("verification_route_task_class"),
        verification_task_class_for(task_class, write_scope),
    )
    independent_verification_required = policy_value(
        task_route_cfg.get("independent_verification_required"),
        "yes" if independent_verification_required_for(task_class, write_scope, dispatch_required) else "no",
    )
    excluded = excluded_subagents or set()
    candidates: list[dict[str, Any]] = []
    suppressed_subagents: list[dict[str, Any]] = []
    for idx, subagent in enumerate(subagent_order):
        if subagent in excluded:
            suppressed_subagents.append({"subagent": subagent, "reason": "excluded_for_independent_verification"})
            continue
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
        lifecycle_stage = lifecycle_stage_for(payload, global_card, scoring_cfg)
        lane_stage = lane_lifecycle_stage_for(task_card, lifecycle_stage, scoring_cfg)
        task_state = policy_value(task_card.get("state"), "")
        global_state = policy_value(global_card.get("state"), "normal")
        recovery_adjustment, recovery_reasons, recovered_recently = recovery_routing_adjustment(global_card, task_card)
        if lifecycle_stage == "retired":
            suppressed_subagents.append({"subagent": subagent, "reason": "retired"})
            continue
        if lane_stage == "probation" and risk_class != "R0":
            suppressed_subagents.append({"subagent": subagent, "reason": "probation_lane_restricted"})
            continue
        if task_state == "demoted" and not recovered_recently:
            suppressed_subagents.append({"subagent": subagent, "reason": "task_class_demoted"})
            continue
        if (
            global_state == "demoted"
            and task_state != "preferred"
            and subagent not in {bridge_fallback_subagent, "internal_subagents"}
            and not recovered_recently
        ):
            suppressed_subagents.append({"subagent": subagent, "reason": "globally_demoted"})
            continue
        useful_progress_rate = float(task_card.get("useful_progress_rate", global_card.get("useful_progress_rate", 0)) or 0)
        chatter_only_count = int(task_card.get("chatter_only_count", global_card.get("chatter_only_count", 0)) or 0)
        timeout_after_progress_count = int(
            task_card.get("timeout_after_progress_count", global_card.get("timeout_after_progress_count", 0)) or 0
        )
        startup_timeout_count = int(task_card.get("startup_timeout_count", global_card.get("startup_timeout_count", 0)) or 0)
        no_output_timeout_count = int(task_card.get("no_output_timeout_count", global_card.get("no_output_timeout_count", 0)) or 0)
        stalled_after_progress_count = int(
            task_card.get("stalled_after_progress_count", global_card.get("stalled_after_progress_count", 0)) or 0
        )
        budget_cost_units = subagent_budget_cost_units(payload)
        budget_adjustment, budget_reasons = budget_efficiency_adjustment(
            payload,
            task_card,
            global_card,
            str(budget_limits.get("budget_policy", "balanced")),
            int(budget_limits.get("max_budget_units", 0) or 0),
        )
        state = task_state or global_state
        subagent_state = policy_value(global_card.get("subagent_state"), "active")
        consecutive = int(task_card.get("consecutive_failures", card.get("global", {}).get("consecutive_failures", 0)))
        if state == "demoted" and consecutive >= int(scoring_cfg["consecutive_failure_limit"]):
            continue
        priority_bonus = max(0, 30 - (idx * 10))
        progress_bonus = int(round(useful_progress_rate * 20))
        chatter_penalty = min(20, chatter_only_count * 10)
        timeout_penalty = min(15, timeout_after_progress_count * 8)
        timeout_instability_penalty = min(
            18,
            (startup_timeout_count * 4) + (no_output_timeout_count * 5) + (stalled_after_progress_count * 6),
        )
        fit_bonus, fit_reasons = task_class_fit_bonus(task_class, payload)
        memory_adjustment, memory_reasons = strategy_memory_adjustment(task_class, subagent, strategy)
        effective_score = (
            learned_score
            + priority_bonus
            + progress_bonus
            + fit_bonus
            + memory_adjustment
            + recovery_adjustment
            + budget_adjustment
            - chatter_penalty
            - timeout_penalty
            - timeout_instability_penalty
        )
        selected_model, model_source = selected_model_for_subagent(subagent, payload, task_route_cfg)
        selected_profile, profile_source = selected_profile_for_subagent(subagent, payload, task_route_cfg)
        candidate_runtime = adaptive_runtime_seconds(
            max_runtime_seconds,
            payload,
            task_card,
            global_card,
            effective_score,
        )
        startup_timeout_seconds = max(5, policy_int(payload.get("dispatch", {}).get("startup_timeout_seconds"), 45))
        no_output_timeout_seconds = max(5, policy_int(payload.get("dispatch", {}).get("no_output_timeout_seconds"), 120))
        progress_idle_timeout_seconds = max(5, policy_int(payload.get("dispatch", {}).get("progress_idle_timeout_seconds"), 90))
        candidates.append(
            {
                "effective_score": effective_score,
                "subagent": subagent,
                "state": state,
                "lifecycle_stage": lifecycle_stage,
                "lane_stage": lane_stage,
                "task_fit_score": learned_score,
                "global_score": global_score,
                "task_class_fit_bonus": fit_bonus,
                "task_class_fit_reasons": [*fit_reasons, *memory_reasons, *recovery_reasons, *budget_reasons],
                "memory_adjustment": memory_adjustment,
                "budget_adjustment": budget_adjustment,
                "budget_cost_units": budget_cost_units,
                "success_count": int(task_card.get("success_count", global_card.get("success_count", 0)) or 0),
                "selected_model": selected_model,
                "selected_model_source": model_source,
                "selected_profile": selected_profile,
                "selected_profile_source": profile_source,
                "max_runtime_seconds": candidate_runtime,
                "startup_timeout_seconds": startup_timeout_seconds,
                "no_output_timeout_seconds": no_output_timeout_seconds,
                "progress_idle_timeout_seconds": progress_idle_timeout_seconds,
                "subagent_backend_class": payload.get("subagent_backend_class"),
                "subagent_state": subagent_state,
                "capability_band": payload.get("capability_band", []),
                "subagent_write_scope": payload.get("write_scope", "none"),
                "orchestration_tier": payload.get("orchestration_tier", "standard"),
                "cost_priority": payload.get("cost_priority", "normal"),
                "useful_progress_rate": useful_progress_rate,
                "chatter_only_count": chatter_only_count,
                "timeout_after_progress_count": timeout_after_progress_count,
                "startup_timeout_count": startup_timeout_count,
                "no_output_timeout_count": no_output_timeout_count,
                "stalled_after_progress_count": stalled_after_progress_count,
                "timeout_instability_penalty": timeout_instability_penalty,
                "recovery_adjustment": recovery_adjustment,
                "recovered_recently": recovered_recently,
                "memory_reasons": memory_reasons,
            }
        )
    return {
        "task_class": task_class,
        "task_route_cfg": task_route_cfg,
        "candidates": candidates,
        "suppressed_subagents": suppressed_subagents,
        "write_scope": write_scope,
        "verification_gate": verification_gate,
        "risk_class": risk_class,
        "max_runtime_seconds": max_runtime_seconds,
        "min_output_bytes": min_output_bytes,
        "merge_policy": merge_policy,
        "fanout_order": fanout_order,
        "dispatch_required": dispatch_required,
        "external_first_required": external_first_required,
        "bridge_fallback_subagent": bridge_fallback_subagent,
        "internal_escalation_trigger": internal_escalation_trigger,
        "graph_strategy": graph_strategy,
        "deterministic_first": deterministic_first,
        "max_parallel_agents": max_parallel_agents,
        "state_owner": state_owner,
        "budget_limits": budget_limits,
        "verification_route_task_class": verification_route_task_class,
        "independent_verification_required": independent_verification_required,
    }


def build_independent_verification_plan(
    task_class: str,
    snapshot: dict[str, Any],
    config: dict[str, Any],
    strategy: dict[str, Any],
    excluded_subagents: set[str],
    verification_task_class: str | None = None,
    required: str = "no",
) -> dict[str, Any]:
    verification_task_class = verification_task_class or verification_task_class_for(task_class, "none")
    if not verification_task_class:
        return {
            "required": required,
            "route_task_class": "",
            "selected_subagent": None,
            "fallback_subagents": [],
            "independent": False,
            "reason": "no_verification_route",
        }
    verification_ctx = route_candidate_context(
        verification_task_class,
        snapshot,
        config,
        strategy,
        excluded_subagents=excluded_subagents,
    )
    verification_candidates = verification_ctx["candidates"]
    used_exclusion_fallback = False
    if not verification_candidates:
        verification_ctx = route_candidate_context(
            verification_task_class,
            snapshot,
            config,
            strategy,
            excluded_subagents=set(),
        )
        verification_candidates = verification_ctx["candidates"]
        used_exclusion_fallback = True
    if not verification_candidates:
        return {
            "required": required,
            "route_task_class": verification_task_class,
            "selected_subagent": None,
            "fallback_subagents": [],
            "independent": False,
            "reason": "no_eligible_verifier",
            "target_review_state": target_review_state_for(verification_ctx["risk_class"]),
            "target_manifest_review_state": target_manifest_review_state_for(verification_ctx["risk_class"]),
            "verification_gate": verification_ctx["verification_gate"],
        }
    verification_candidates.sort(key=lambda item: int(item["effective_score"]), reverse=True)
    verification_candidates = apply_bridge_fallback_priority(
        verification_candidates,
        verification_ctx["bridge_fallback_subagent"],
    )
    selected = verification_candidates[0]
    return {
        "required": required,
        "route_task_class": verification_task_class,
        "selected_subagent": selected["subagent"],
        "selected_model": selected["selected_model"],
        "selected_profile": selected["selected_profile"],
        "selected_model_source": selected["selected_model_source"],
        "selected_profile_source": selected["selected_profile_source"],
        "effective_score": selected["effective_score"],
        "target_review_state": target_review_state_for(verification_ctx["risk_class"]),
        "target_manifest_review_state": target_manifest_review_state_for(verification_ctx["risk_class"]),
        "verification_gate": verification_ctx["verification_gate"],
        "independent": not used_exclusion_fallback and selected["subagent"] not in excluded_subagents,
        "reason": "independent_verifier_selected" if not used_exclusion_fallback else "same_lane_verifier_fallback",
        "fallback_subagents": [
            {
                "subagent": item["subagent"],
                "effective_score": item["effective_score"],
                "selected_model": item["selected_model"],
                "selected_profile": item["selected_profile"],
            }
            for item in verification_candidates[1:]
        ],
    }


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
    strategy = load_strategy_memory()
    context = route_candidate_context(task_class, snapshot, config, strategy)
    task_route_cfg = context["task_route_cfg"]
    candidates = context["candidates"]
    suppressed_subagents = context["suppressed_subagents"]
    write_scope = context["write_scope"]
    verification_gate = context["verification_gate"]
    risk_class = context["risk_class"]
    max_runtime_seconds = context["max_runtime_seconds"]
    min_output_bytes = context["min_output_bytes"]
    merge_policy = context["merge_policy"]
    fanout_order = context["fanout_order"]
    dispatch_required = context["dispatch_required"]
    external_first_required = context["external_first_required"]
    bridge_fallback_subagent = context["bridge_fallback_subagent"]
    internal_escalation_trigger = context["internal_escalation_trigger"]
    graph_strategy = context["graph_strategy"]
    deterministic_first = context["deterministic_first"]
    max_parallel_agents = context["max_parallel_agents"]
    state_owner = context["state_owner"]
    budget_limits = context["budget_limits"]
    verification_route_task_class = context["verification_route_task_class"]
    independent_verification_required = context["independent_verification_required"]

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
            "target_review_state": target_review_state_for(risk_class),
            "target_manifest_review_state": target_manifest_review_state_for(risk_class),
            "max_runtime_seconds": max_runtime_seconds,
            "min_output_bytes": min_output_bytes,
            "fanout_subagents": [],
            "fanout_min_results": 0,
            "merge_policy": merge_policy,
            "dispatch_required": dispatch_required,
            "external_first_required": external_first_required,
            "bridge_fallback_subagent": bridge_fallback_subagent,
            "internal_escalation_trigger": internal_escalation_trigger,
            "graph_strategy": graph_strategy,
            "deterministic_first": deterministic_first,
            "route_budget": budget_limits,
            "route_graph": {
                "graph_strategy": graph_strategy,
                "deterministic_first": deterministic_first,
                "primary_mode": "none",
                "nodes": [],
                "edges": [],
                "planned_path": [],
            },
            "verification_route_task_class": verification_route_task_class,
            "independent_verification_required": independent_verification_required,
            "verification_plan": {
                "required": independent_verification_required,
                "route_task_class": verification_route_task_class,
                "selected_subagent": None,
                "fallback_subagents": [],
                "independent": False,
                "reason": "no_primary_route_available",
            },
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
    verification_plan = build_independent_verification_plan(
        task_class,
        snapshot,
        config,
        strategy,
        {selected["subagent"], *fanout_subagents},
        verification_route_task_class,
        independent_verification_required,
    )
    route_budget = build_route_budget(
        budget_limits=budget_limits,
        selected=selected,
        fanout_subagents=fanout_subagents,
        verification_plan=verification_plan,
        bridge_fallback_subagent=bridge_fallback_subagent,
    )
    route_graph = build_route_graph(
        task_class=task_class,
        dispatch_required=dispatch_required,
        deterministic_first=deterministic_first,
        graph_strategy=graph_strategy,
        selected_subagent=selected["subagent"],
        fanout_subagents=fanout_subagents,
        verification_plan=verification_plan,
        bridge_fallback_subagent=bridge_fallback_subagent,
        internal_escalation_trigger=internal_escalation_trigger,
        budget=route_budget,
    )
    return {
        "task_class": task_class,
        "selected_subagent": selected["subagent"],
        "reason": f"state={selected['state']}",
        "lifecycle_stage": selected["lifecycle_stage"],
        "lane_stage": selected["lane_stage"],
        "effective_score": selected["effective_score"],
        "task_fit_score": selected["task_fit_score"],
        "global_score": selected["global_score"],
        "task_class_fit_bonus": selected["task_class_fit_bonus"],
        "task_class_fit_reasons": selected["task_class_fit_reasons"],
        "memory_adjustment": selected["memory_adjustment"],
        "budget_adjustment": selected["budget_adjustment"],
        "budget_cost_units": selected["budget_cost_units"],
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
        "target_review_state": target_review_state_for(risk_class),
        "target_manifest_review_state": target_manifest_review_state_for(risk_class),
        "max_runtime_seconds": selected["max_runtime_seconds"],
        "startup_timeout_seconds": selected["startup_timeout_seconds"],
        "no_output_timeout_seconds": selected["no_output_timeout_seconds"],
        "progress_idle_timeout_seconds": selected["progress_idle_timeout_seconds"],
        "min_output_bytes": min_output_bytes,
        "fanout_subagents": fanout_subagents,
        "fanout_min_results": fanout_min_results,
        "merge_policy": merge_policy,
        "dispatch_required": dispatch_required,
        "external_first_required": external_first_required,
        "bridge_fallback_subagent": bridge_fallback_subagent,
        "internal_escalation_trigger": internal_escalation_trigger,
        "graph_strategy": graph_strategy,
        "deterministic_first": deterministic_first,
        "route_budget": route_budget,
        "route_graph": route_graph,
        "verification_route_task_class": verification_route_task_class,
        "independent_verification_required": independent_verification_required,
        "verification_plan": verification_plan,
        "max_parallel_agents": max_parallel_agents,
        "state_owner": state_owner,
        "suppressed_subagents": suppressed_subagents,
        "fallback_subagents": [
            {
                "subagent": item["subagent"],
                "effective_score": item["effective_score"],
                "lifecycle_stage": item["lifecycle_stage"],
                "lane_stage": item["lane_stage"],
                "task_fit_score": item["task_fit_score"],
                "global_score": item["global_score"],
                "task_class_fit_bonus": item["task_class_fit_bonus"],
                "task_class_fit_reasons": item["task_class_fit_reasons"],
                "memory_adjustment": item["memory_adjustment"],
                "budget_adjustment": item["budget_adjustment"],
                "budget_cost_units": item["budget_cost_units"],
                "selected_model": item["selected_model"],
                "selected_model_source": item["selected_model_source"],
                "selected_profile": item["selected_profile"],
                "selected_profile_source": item["selected_profile_source"],
                "max_runtime_seconds": item["max_runtime_seconds"],
                "startup_timeout_seconds": item["startup_timeout_seconds"],
                "no_output_timeout_seconds": item["no_output_timeout_seconds"],
                "progress_idle_timeout_seconds": item["progress_idle_timeout_seconds"],
                "subagent_state": item["subagent_state"],
                "orchestration_tier": item["orchestration_tier"],
                "cost_priority": item["cost_priority"],
                "target_review_state": target_review_state_for(risk_class),
                "target_manifest_review_state": target_manifest_review_state_for(risk_class),
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
    normalized_domain_tags = normalize_domain_tags(domain_tags)
    domain_cards = [domain_buckets.setdefault(tag, dict(global_card)) for tag in normalized_domain_tags]
    metrics = metrics or {}
    useful_progress = bool(metrics.get("useful_progress", False))
    timeout_after_progress = bool(metrics.get("timeout_after_progress", False))
    chatter_only = bool(metrics.get("chatter_only", False))
    time_to_first_useful_output_ms = metrics.get("time_to_first_useful_output_ms")
    failure_reason = policy_value(metrics.get("failure_reason"), "")
    verification_role = policy_value(metrics.get("verification_role"), "")
    independent_verification_passed = bool(metrics.get("independent_verification_passed", False))
    independent_verification_failed = bool(metrics.get("independent_verification_failed", False))
    verification_caught_issue = bool(metrics.get("verification_caught_issue", False))

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

    if verification_role == "author":
        for bucket in (global_card, task_card, *domain_cards):
            bucket["authored_runs_count"] = int(bucket.get("authored_runs_count", 0)) + 1
            if independent_verification_passed:
                bucket["authored_verified_pass_count"] = int(bucket.get("authored_verified_pass_count", 0)) + 1
            if independent_verification_failed:
                bucket["authored_verified_fail_count"] = int(bucket.get("authored_verified_fail_count", 0)) + 1
        if independent_verification_passed:
            delta += 5
        if independent_verification_failed:
            delta -= 10
    elif verification_role == "verifier":
        for bucket in (global_card, task_card, *domain_cards):
            bucket["verifier_runs_count"] = int(bucket.get("verifier_runs_count", 0)) + 1
            if result == "success":
                bucket["verifier_success_count"] = int(bucket.get("verifier_success_count", 0)) + 1
            if verification_caught_issue:
                bucket["verifier_catch_count"] = int(bucket.get("verifier_catch_count", 0)) + 1
        if verification_caught_issue:
            delta += 6
        elif result == "success":
            delta += 4

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
        if failure_reason == "startup_timeout":
            bucket["startup_timeout_count"] = int(bucket.get("startup_timeout_count", 0)) + 1
        if failure_reason == "no_output_timeout":
            bucket["no_output_timeout_count"] = int(bucket.get("no_output_timeout_count", 0)) + 1
        if failure_reason == "stalled_after_progress":
            bucket["stalled_after_progress_count"] = int(bucket.get("stalled_after_progress_count", 0)) + 1
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
    global_card["lifecycle_stage"] = lifecycle_stage_for(snapshot.get("subagents", {}).get(subagent, {}), global_card, scoring_cfg)
    task_card["lifecycle_stage"] = lane_lifecycle_stage_for(task_card, global_card["lifecycle_stage"], scoring_cfg)

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
        "  python3 _vida/scripts/subagent-system.py diagnose [task_class]\n"
        "  python3 _vida/scripts/subagent-system.py route <task_class>\n"
        "  python3 _vida/scripts/subagent-system.py probe <subagent>\n"
        "  python3 _vida/scripts/subagent-system.py recover <subagent>\n"
        "  python3 _vida/scripts/subagent-system.py recover-pending\n"
        "  python3 _vida/scripts/subagent-system.py leases\n"
        "  python3 _vida/scripts/subagent-system.py lease-renew <resource_type> <resource_id> <holder> [ttl_seconds]\n"
        "  python3 _vida/scripts/subagent-system.py lease-cleanup\n"
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
        if cmd == "diagnose":
            task_class = argv[2] if len(argv) > 2 else None
            print(json.dumps(subagent_diagnosis(task_class), indent=2, sort_keys=True))
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
        if cmd == "leases":
            print(json.dumps(active_leases(), indent=2, sort_keys=True))
            return 0
        if cmd == "lease-renew":
            if len(argv) < 5:
                usage()
                return 1
            ttl_seconds = int(argv[5]) if len(argv) > 5 else 3600
            print(json.dumps(renew_lease(argv[2], argv[3], argv[4], ttl_seconds), indent=2, sort_keys=True))
            return 0
        if cmd == "lease-cleanup":
            payload = cleanup_leases(load_leases())
            save_leases(payload)
            print(json.dumps(active_leases(), indent=2, sort_keys=True))
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
        config = vida_config.load_validated_config()
        payload = load_scorecards(detect_subagents(config))
        save_json(SCORECARD_PATH, payload)
        if len(argv) > 2:
            print(json.dumps(payload.get("subagents", {}).get(argv[2], {}), indent=2, sort_keys=True))
        else:
            print(json.dumps(payload, indent=2, sort_keys=True))
        return 0
    usage()
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
