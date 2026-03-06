#!/usr/bin/env python3
"""Generic subagent-system runtime helper for VIDA."""

from __future__ import annotations

import importlib.util
import json
import shutil
import sys
from datetime import datetime, timezone
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


def ensure_state_dir() -> None:
    STATE_DIR.mkdir(parents=True, exist_ok=True)


def load_json(path: Path, default: Any) -> Any:
    if not path.exists():
        return default
    try:
        return json.loads(path.read_text())
    except Exception:
        return default


def save_json(path: Path, payload: Any) -> None:
    ensure_state_dir()
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


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


def models_hint_for(provider_name: str, provider_cfg: dict[str, Any]) -> list[str]:
    hint = provider_cfg.get("models_hint")
    if isinstance(hint, str):
        return split_csv(hint)
    if provider_name != "codex_cli":
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
    normalized_gate = policy_value(verification_gate, "provider_return_contract")
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


def detect_providers(config: dict[str, Any]) -> dict[str, Any]:
    providers = vida_config.dotted_get(config, "agent_system.providers", {}) or {}
    detected: dict[str, Any] = {}
    for name, provider_cfg in providers.items():
        if not isinstance(provider_cfg, dict):
            continue
        enabled = bool(provider_cfg.get("enabled", False))
        provider_class = provider_cfg.get("provider_class", "external_cli")
        detect_command = provider_cfg.get("detect_command")
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
            "provider_class": provider_class,
            "role": provider_cfg.get("role", "secondary"),
            "orchestration_tier": policy_value(provider_cfg.get("orchestration_tier"), "standard"),
            "cost_priority": policy_value(provider_cfg.get("cost_priority"), "normal"),
            "detect_command": detect_command,
            "models_hint": models_hint_for(name, provider_cfg),
            "default_model": provider_cfg.get("default_model"),
            "profiles": split_csv(provider_cfg.get("profiles")),
            "default_profile": provider_cfg.get("default_profile"),
            "capability_band": split_csv(provider_cfg.get("capability_band")),
            "write_scope": policy_value(provider_cfg.get("write_scope"), "none"),
            "billing_tier": policy_value(provider_cfg.get("billing_tier"), "unknown"),
            "speed_tier": policy_value(provider_cfg.get("speed_tier"), "unknown"),
            "quality_tier": policy_value(provider_cfg.get("quality_tier"), "unknown"),
            "specialties": split_csv(provider_cfg.get("specialties")),
            "dispatch": provider_cfg.get("dispatch", {}) if isinstance(provider_cfg.get("dispatch"), dict) else {},
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


def effective_mode(config: dict[str, Any], providers: dict[str, Any]) -> tuple[str, list[str]]:
    protocol_active = bool(vida_config.dotted_get(config, "protocol_activation.agent_system", False))
    if not protocol_active:
        return "disabled", ["protocol_activation.agent_system=false"]

    requested = str(vida_config.dotted_get(config, "agent_system.mode", "native"))
    has_internal = bool(providers.get("internal_subagents", {}).get("available"))
    has_external = any(
        name != "internal_subagents" and payload.get("available")
        for name, payload in providers.items()
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
            return "native", ["requested_mode=hybrid", "external providers unavailable -> degrade_to=native"]
        if has_external:
            return "disabled", ["requested_mode=hybrid", "internal providers unavailable -> degrade_to=disabled"]
        return "disabled", ["requested_mode=hybrid", "no providers available"]
    return "disabled", [f"unsupported requested_mode={requested}"]


def score_defaults() -> dict[str, Any]:
    return {
        "global": {
            "score": 50,
            "success_count": 0,
            "failure_count": 0,
            "consecutive_failures": 0,
            "state": "normal",
        },
        "by_task_class": {},
        "by_domain": {},
    }


def load_scorecards(providers: dict[str, Any]) -> dict[str, Any]:
    payload = load_json(SCORECARD_PATH, {"providers": {}})
    provider_payload = payload.setdefault("providers", {})
    for name in providers:
        provider_payload.setdefault(name, score_defaults())
    return payload


def classify_state(score: int, consecutive_failures: int, cfg: dict[str, int]) -> str:
    if consecutive_failures >= cfg["consecutive_failure_limit"]:
        return "demoted"
    if score >= cfg["promotion_score"]:
        return "preferred"
    return "normal"


def init_snapshot(task_id: str | None = None) -> dict[str, Any]:
    config = vida_config.load_validated_config()
    provider_state = detect_providers(config)
    scoring_cfg = thresholds(config)
    mode, reasons = effective_mode(config, provider_state)
    scorecards = load_scorecards(provider_state)
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
        "providers": provider_state,
        "scorecards": scorecards["providers"],
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


def selected_model_for(provider: str, provider_cfg: dict[str, Any], route_cfg: dict[str, Any]) -> tuple[str | None, str]:
    route_model = route_models(route_cfg).get(provider)
    if route_model:
        return route_model, "route_override"
    default_model = provider_cfg.get("default_model")
    if isinstance(default_model, str) and default_model:
        return default_model, "provider_default"
    return None, "none"


def selected_profile_for(provider: str, provider_cfg: dict[str, Any], route_cfg: dict[str, Any]) -> tuple[str | None, str]:
    route_profile = route_profiles(route_cfg).get(provider)
    available_profiles = provider_cfg.get("profiles", [])
    if route_profile and route_profile in available_profiles:
        return route_profile, "route_override"
    default_profile = provider_cfg.get("default_profile")
    if isinstance(default_profile, str) and default_profile:
        return default_profile, "provider_default"
    return None, "none"


def adaptive_runtime_seconds(
    base_limit: int,
    provider_cfg: dict[str, Any],
    task_card: dict[str, Any],
    global_card: dict[str, Any],
    effective_score: int,
) -> int:
    provider_limit = policy_int(provider_cfg.get("max_runtime_seconds"), 0)
    baseline = base_limit or provider_limit or 180
    latency_ms = policy_int(task_card.get("last_latency_ms"), 0) or policy_int(global_card.get("last_latency_ms"), 0)
    last_result = policy_value(task_card.get("last_result"), policy_value(global_card.get("last_result"), ""))
    quality_tier = policy_value(provider_cfg.get("quality_tier"), "medium")
    speed_tier = policy_value(provider_cfg.get("speed_tier"), "medium")

    budget = baseline
    if latency_ms > 0 and last_result == "success":
        learned_seconds = int((latency_ms / 1000.0) * 1.2) + 20
        budget = max(budget, learned_seconds)
    elif latency_ms > 0 and last_result == "failure":
        budget = max(budget, int((latency_ms / 1000.0) * 0.9))

    if quality_tier == "high" and effective_score >= 65:
        budget += 20
    if speed_tier == "fast":
        budget = min(budget, baseline + 20)

    return max(120, min(300, budget))


def apply_bridge_fallback_priority(
    candidates: list[dict[str, Any]],
    bridge_fallback_provider: str,
) -> list[dict[str, Any]]:
    if not bridge_fallback_provider or len(candidates) < 2:
        return candidates
    bridge_index = next(
        (idx for idx, item in enumerate(candidates) if item.get("provider") == bridge_fallback_provider),
        -1,
    )
    if bridge_index <= 1:
        return candidates
    bridge_item = candidates[bridge_index]
    remaining = [item for idx, item in enumerate(candidates) if idx != bridge_index]
    selected = remaining[:1]
    tail = remaining[1:]
    internal_items = [item for item in tail if item.get("provider_class") == "internal"]
    external_items = [item for item in tail if item.get("provider_class") != "internal"]
    return [*selected, bridge_item, *external_items, *internal_items]


def route_provider(task_class: str) -> dict[str, Any]:
    snapshot = load_json(INIT_PATH, {})
    if not snapshot:
        snapshot = init_snapshot()
    if snapshot.get("agent_system", {}).get("effective_mode") == "disabled":
        return {
            "task_class": task_class,
            "provider": None,
            "reason": "subagent system disabled",
            "effective_score": 0,
        }

    config = vida_config.load_validated_config()
    task_route_cfg = route_config_for(config, task_class)
    provider_order = split_csv(task_route_cfg.get("providers"))
    if not provider_order:
        provider_order = split_csv(vida_config.dotted_get(config, "agent_system.routing.default.providers", ""))
    providers = snapshot.get("providers", {})
    scores = snapshot.get("scorecards", {})
    scoring_cfg = snapshot.get("agent_system", {}).get("scoring", thresholds(config))
    mode = snapshot.get("agent_system", {}).get("effective_mode")
    max_parallel_agents = int(snapshot.get("agent_system", {}).get("max_parallel_agents", 1))
    state_owner = str(snapshot.get("agent_system", {}).get("state_owner", "orchestrator_only"))
    write_scope = policy_value(task_route_cfg.get("write_scope"), "none")
    verification_gate = policy_value(task_route_cfg.get("verification_gate"), "provider_return_contract")
    risk_class = inferred_risk_class(task_class, write_scope, verification_gate)
    max_runtime_seconds = policy_int(task_route_cfg.get("max_runtime_seconds"), 0)
    min_output_bytes = policy_int(task_route_cfg.get("min_output_bytes"), 0)
    merge_policy = policy_value(task_route_cfg.get("merge_policy"), "single_provider")
    fanout_order = split_csv(task_route_cfg.get("fanout_providers"))
    dispatch_required = policy_value(task_route_cfg.get("dispatch_required"), "optional")
    external_first_required = policy_value(task_route_cfg.get("external_first_required"), "no")
    bridge_fallback_provider = policy_value(task_route_cfg.get("bridge_fallback_provider"), "")
    internal_escalation_trigger = policy_value(task_route_cfg.get("internal_escalation_trigger"), "")

    candidates: list[dict[str, Any]] = []
    for idx, provider in enumerate(provider_order):
        payload = providers.get(provider, {})
        if not payload.get("enabled") or not payload.get("available"):
            continue
        if mode == "native" and payload.get("provider_class") != "internal":
            continue
        card = scores.get(provider, score_defaults())
        global_card = card.get("global", {})
        task_card = card.get("by_task_class", {}).get(task_class, {})
        learned_score = int(task_card.get("score", card.get("global", {}).get("score", 50)))
        state = task_card.get("state", card.get("global", {}).get("state", "normal"))
        consecutive = int(task_card.get("consecutive_failures", card.get("global", {}).get("consecutive_failures", 0)))
        if state == "demoted" and consecutive >= int(scoring_cfg["consecutive_failure_limit"]):
            continue
        priority_bonus = max(0, 30 - (idx * 10))
        effective_score = learned_score + priority_bonus
        selected_model, model_source = selected_model_for(provider, payload, task_route_cfg)
        selected_profile, profile_source = selected_profile_for(provider, payload, task_route_cfg)
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
                "provider": provider,
                "state": state,
                "selected_model": selected_model,
                "selected_model_source": model_source,
                "selected_profile": selected_profile,
                "selected_profile_source": profile_source,
                "max_runtime_seconds": candidate_runtime,
                "provider_class": payload.get("provider_class"),
                "capability_band": payload.get("capability_band", []),
                "provider_write_scope": payload.get("write_scope", "none"),
                "orchestration_tier": payload.get("orchestration_tier", "standard"),
                "cost_priority": payload.get("cost_priority", "normal"),
            }
        )

    if not candidates:
        return {
            "task_class": task_class,
            "provider": None,
            "reason": "no eligible providers after mode/capability/score filtering",
            "effective_score": 0,
            "selected_model": None,
            "selected_model_source": "none",
            "selected_profile": None,
            "selected_profile_source": "none",
            "fallback_chain": [],
            "write_scope": write_scope,
            "verification_gate": verification_gate,
            "risk_class": risk_class,
            "max_runtime_seconds": max_runtime_seconds,
            "min_output_bytes": min_output_bytes,
            "fanout_providers": [],
            "fanout_min_results": 0,
            "merge_policy": merge_policy,
            "dispatch_required": dispatch_required,
            "external_first_required": external_first_required,
            "bridge_fallback_provider": bridge_fallback_provider,
            "internal_escalation_trigger": internal_escalation_trigger,
            "max_parallel_agents": max_parallel_agents,
            "state_owner": state_owner,
        }

    candidates.sort(key=lambda item: int(item["effective_score"]), reverse=True)
    candidates = apply_bridge_fallback_priority(candidates, bridge_fallback_provider)
    selected = candidates[0]
    eligible_providers = {item["provider"] for item in candidates}
    fanout_providers = [provider for provider in fanout_order if provider in eligible_providers]
    default_fanout_min = min(2, len(fanout_providers)) if fanout_providers else 0
    fanout_min_results = max(
        0,
        min(
            policy_int(task_route_cfg.get("fanout_min_results"), default_fanout_min),
            len(fanout_providers),
        ),
    )
    return {
        "task_class": task_class,
        "provider": selected["provider"],
        "reason": f"state={selected['state']}",
        "effective_score": selected["effective_score"],
        "selected_model": selected["selected_model"],
        "selected_model_source": selected["selected_model_source"],
        "selected_profile": selected["selected_profile"],
        "selected_profile_source": selected["selected_profile_source"],
        "provider_class": selected["provider_class"],
        "capability_band": selected["capability_band"],
        "provider_write_scope": selected["provider_write_scope"],
        "orchestration_tier": selected["orchestration_tier"],
        "cost_priority": selected["cost_priority"],
        "write_scope": write_scope,
        "verification_gate": verification_gate,
        "risk_class": risk_class,
        "max_runtime_seconds": selected["max_runtime_seconds"],
        "min_output_bytes": min_output_bytes,
        "fanout_providers": fanout_providers,
        "fanout_min_results": fanout_min_results,
        "merge_policy": merge_policy,
        "dispatch_required": dispatch_required,
        "external_first_required": external_first_required,
        "bridge_fallback_provider": bridge_fallback_provider,
        "internal_escalation_trigger": internal_escalation_trigger,
        "max_parallel_agents": max_parallel_agents,
        "state_owner": state_owner,
        "fallback_chain": [
            {
                "provider": item["provider"],
                "effective_score": item["effective_score"],
                "selected_model": item["selected_model"],
                "selected_model_source": item["selected_model_source"],
                "selected_profile": item["selected_profile"],
                "selected_profile_source": item["selected_profile_source"],
                "max_runtime_seconds": item["max_runtime_seconds"],
                "orchestration_tier": item["orchestration_tier"],
                "cost_priority": item["cost_priority"],
            }
            for item in candidates[1:]
        ],
    }


def update_score(
    provider: str,
    result: str,
    task_class: str,
    quality_score: int,
    latency_ms: int,
    note: str,
    domain_tags: list[str] | None = None,
) -> dict[str, Any]:
    snapshot = load_json(INIT_PATH, {})
    if not snapshot:
        snapshot = init_snapshot()
    config = vida_config.load_validated_config()
    scoring_cfg = thresholds(config)
    scorecards = load_json(SCORECARD_PATH, {"providers": {}})
    provider_cards = scorecards.setdefault("providers", {})
    card = provider_cards.setdefault(provider, score_defaults())
    global_card = card.setdefault("global", score_defaults()["global"])
    task_card = card.setdefault("by_task_class", {}).setdefault(task_class, dict(global_card))
    domain_buckets = card.setdefault("by_domain", {})
    normalized_domain_tags = [tag for tag in (domain_tags or []) if tag]
    domain_cards = [domain_buckets.setdefault(tag, dict(global_card)) for tag in normalized_domain_tags]

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
        bucket["updated_at"] = now_utc()

    save_json(SCORECARD_PATH, scorecards)
    snapshot["scorecards"] = scorecards["providers"]
    snapshot["written_at"] = now_utc()
    save_json(INIT_PATH, snapshot)
    return {
        "provider": provider,
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
        "  python3 _vida/scripts/subagent-system.py route <task_class>\n"
        "  python3 _vida/scripts/subagent-system.py record <provider> <success|failure> <task_class> [quality_score] [latency_ms] [note]\n"
        "  python3 _vida/scripts/subagent-system.py scorecard [provider]",
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
            payload = load_json(INIT_PATH, {})
            if not payload:
                payload = init_snapshot()
            print(json.dumps(payload, indent=2, sort_keys=True))
            return 0
        if cmd == "route":
            if len(argv) < 3:
                usage()
                return 1
            print(json.dumps(route_provider(argv[2]), indent=2, sort_keys=True))
            return 0
        if cmd == "record":
            if len(argv) < 5:
                usage()
                return 1
            provider = argv[2]
            result = argv[3]
            task_class = argv[4]
            quality_score = int(argv[5]) if len(argv) > 5 else (85 if result == "success" else 20)
            latency_ms = int(argv[6]) if len(argv) > 6 else 0
            note = argv[7] if len(argv) > 7 else ""
            print(json.dumps(update_score(provider, result, task_class, quality_score, latency_ms, note), indent=2, sort_keys=True))
            return 0
    except (ValueError, vida_config.OverlayValidationError) as exc:
        print(f"[subagent-system] {exc}", file=sys.stderr)
        return 2
    if cmd == "scorecard":
        payload = load_json(SCORECARD_PATH, {"providers": {}})
        if len(argv) > 2:
            print(json.dumps(payload.get("providers", {}).get(argv[2], {}), indent=2, sort_keys=True))
        else:
            print(json.dumps(payload, indent=2, sort_keys=True))
        return 0
    usage()
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
