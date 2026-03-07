#!/usr/bin/env python3
"""Post-task subagent evaluation and strategy refresh for VIDA."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
LOG_DIR = ROOT_DIR / ".vida" / "logs"
STATE_DIR = ROOT_DIR / ".vida" / "state"
RUN_LOG_PATH = LOG_DIR / "subagent-runs.jsonl"
PROCESSED_PATH = STATE_DIR / "subagent-eval-processed.json"
STRATEGY_PATH = STATE_DIR / "subagent-strategy.json"


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


subagent_system = load_module("subagent_system_runtime_eval", SCRIPT_DIR / "subagent-system.py")
vida_config = load_module("vida_config_runtime_eval", SCRIPT_DIR / "vida-config.py")


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def load_json(path: Path, default: Any) -> Any:
    if not path.exists():
        return default
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return default


def save_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    out: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(payload, dict):
            out.append(payload)
    return out


def ensure_eval_pack(task_id: str) -> dict[str, Any]:
    out_path = LOG_DIR / f"eval-pack-{task_id}.json"
    completed = subprocess.run(
        ["bash", "_vida/scripts/eval-pack.sh", "run", task_id],
        cwd=str(ROOT_DIR),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if completed.returncode != 0 and not out_path.exists():
        raise RuntimeError(completed.stderr.strip() or "eval-pack.sh failed")
    return load_json(out_path, {})


def task_closed(task_id: str) -> bool:
    completed = subprocess.run(
        ["br", "show", task_id, "--json"],
        cwd=str(ROOT_DIR),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        return False
    payload = json.loads(completed.stdout)
    if isinstance(payload, list) and payload:
        status = payload[0].get("status")
    elif isinstance(payload, dict):
        status = payload.get("status")
    else:
        status = None
    return status == "closed"


def quality_score_for(run: dict[str, Any], eval_pack: dict[str, Any], is_closed: bool) -> int:
    score = 18
    if run.get("status") == "success" and run.get("merge_ready") is True:
        score = 78
    elif run.get("status") == "success":
        score = 38
    elif run.get("status") == "timeout":
        score = 12
    if run.get("useful_progress"):
        score += 10
    if run.get("chatter_only"):
        score -= 12
    if run.get("status") == "timeout" and run.get("useful_progress"):
        score += 6
    if is_closed:
        score += 8
    else:
        score -= 8
    block_success_rate = float(eval_pack.get("block_success_rate", 0) or 0)
    if block_success_rate >= 80:
        score += 8
    elif block_success_rate < 50:
        score -= 10

    duration_ms = int(run.get("duration_ms", 0) or 0)
    if run.get("merge_ready") is True and duration_ms <= 30000:
        score += 6
    elif run.get("merge_ready") is True and duration_ms <= 120000:
        score += 3
    elif duration_ms > 240000:
        score -= 8
    ttfu_ms = int(run.get("time_to_first_useful_output_ms", 0) or 0)
    if ttfu_ms > 0 and ttfu_ms <= 120000:
        score += 4
    elif ttfu_ms > 240000:
        score -= 4

    if run.get("billing_tier") == "free":
        score += 4
    elif run.get("billing_tier") == "paid":
        score -= 2

    if run.get("dispatch_mode") == "fanout":
        score += 2
    if run.get("verification_role") == "author" and run.get("independent_verification_passed"):
        score += 6
    if run.get("verification_role") == "author" and run.get("independent_verification_failed"):
        score -= 10
    if run.get("verification_role") == "verifier":
        score += 4
    if run.get("verification_caught_issue"):
        score += 6

    return max(0, min(100, score))


def infer_domain_tags(run: dict[str, Any]) -> list[str]:
    existing = run.get("domain_tags")
    if isinstance(existing, list):
        return [str(tag) for tag in existing if str(tag).strip()]
    prompt_path = Path(str(run.get("prompt_file", "")))
    task_class = str(run.get("task_class", "")).strip()
    if not prompt_path.exists():
        return [task_class] if task_class else []
    text = prompt_path.read_text(encoding="utf-8", errors="ignore").casefold()
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
    if not tags and task_class:
        tags.append(task_class)
    return list(dict.fromkeys(tags))


def strengths_for_subagent(subagent_name: str, subagent_cfg: dict[str, Any], scorecard: dict[str, Any]) -> list[str]:
    strengths: list[str] = []
    if subagent_cfg.get("billing_tier") == "free":
        strengths.append("zero-cost lane")
    if subagent_cfg.get("speed_tier") == "fast":
        strengths.append("fast turnaround")
    if subagent_cfg.get("quality_tier") == "high":
        strengths.append("high-quality review lane")
    if int(scorecard.get("success_count", 0)) >= int(scorecard.get("failure_count", 0)):
        strengths.append("stable recent outcomes")
    strengths.extend(subagent_cfg.get("specialties", []))
    return list(dict.fromkeys(strengths))[:5]


def budget_efficiency_score(subagent_cfg: dict[str, Any], scorecard: dict[str, Any]) -> int:
    cost_units = subagent_system.subagent_budget_cost_units(subagent_cfg)
    useful_progress_rate = float(scorecard.get("useful_progress_rate", 0) or 0)
    avg_ttfu_ms = int(scorecard.get("avg_time_to_first_useful_output_ms", 0) or 0)
    score = 50
    score += max(0, int(round(useful_progress_rate * 30)))
    score -= min(24, cost_units * 6)
    if avg_ttfu_ms > 0 and avg_ttfu_ms <= 120000:
        score += 8
    elif avg_ttfu_ms > 240000:
        score -= 8
    if subagent_cfg.get("billing_tier") == "free":
        score += 8
    return max(0, min(100, score))


def weaknesses_for_subagent(subagent_name: str, subagent_cfg: dict[str, Any], scorecard: dict[str, Any]) -> list[str]:
    weaknesses: list[str] = []
    if subagent_cfg.get("write_scope") == "none":
        weaknesses.append("read-only only")
    if subagent_cfg.get("billing_tier") in {"low", "paid"}:
        weaknesses.append("cost-limited lane")
    if subagent_cfg.get("default_model") in {None, ""}:
        weaknesses.append("backend resolved outside repo config")
    if int(scorecard.get("score", 50)) < 45:
        weaknesses.append("weak recent confidence trend")
    return list(dict.fromkeys(weaknesses))[:4]


def prompt_family_for_run(run: dict[str, Any]) -> str:
    prompt_path = Path(str(run.get("prompt_file", "")))
    if prompt_path.name:
        return prompt_path.stem
    return str(run.get("task_class", "default")) or "default"


def build_memory_hints(runs: list[dict[str, Any]]) -> tuple[dict[str, Any], dict[str, Any]]:
    grouped: dict[str, dict[str, list[dict[str, Any]]]] = {}
    subagent_signatures: dict[str, dict[str, int]] = {}
    for run in runs:
        task_class = str(run.get("task_class", "") or "default")
        subagent = str(run.get("subagent", "") or "")
        if not subagent:
            continue
        grouped.setdefault(task_class, {}).setdefault(subagent, []).append(run)
        failure_reason = str(run.get("failure_reason", "") or "").strip()
        if failure_reason:
            subagent_signatures.setdefault(subagent, {})
            subagent_signatures[subagent][failure_reason] = subagent_signatures[subagent].get(failure_reason, 0) + 1

    task_class_memory: dict[str, Any] = {}
    for task_class, by_subagent in grouped.items():
        preferred: list[str] = []
        avoid: list[str] = []
        retry_useful: list[str] = []
        failure_prone: list[str] = []
        prompt_families: dict[str, Any] = {}
        for subagent, subagent_runs in by_subagent.items():
            total = len(subagent_runs)
            merge_ready_runs = sum(1 for item in subagent_runs if item.get("merge_ready") is True)
            useful_runs = sum(1 for item in subagent_runs if item.get("useful_progress") is True)
            chatter_runs = sum(1 for item in subagent_runs if item.get("chatter_only") is True)
            timeout_runs = sum(1 for item in subagent_runs if str(item.get("status")) == "timeout")
            merge_rate = merge_ready_runs / max(1, total)
            useful_rate = useful_runs / max(1, total)
            chatter_rate = chatter_runs / max(1, total)
            timeout_rate = timeout_runs / max(1, total)
            if total >= 3 and merge_rate >= 0.75 and useful_rate >= 0.75:
                preferred.append(subagent)
            if total >= 2 and (merge_rate < 0.35 or chatter_rate >= 0.4 or timeout_rate >= 0.5):
                avoid.append(subagent)
            ordered = sorted(
                subagent_runs,
                key=lambda item: (str(item.get("task_id", "")), str(item.get("ts_start", item.get("ts", ""))))
            )
            saw_failure = False
            for item in ordered:
                if str(item.get("status")) in {"timeout", "failure"} or int(item.get("exit_code", 0) or 0) != 0:
                    saw_failure = True
                elif saw_failure and item.get("merge_ready") is True:
                    retry_useful.append(subagent)
                    break
            if total >= 2 and timeout_rate >= 0.5:
                failure_prone.append(subagent)

            family_groups: dict[str, list[dict[str, Any]]] = {}
            for item in subagent_runs:
                family_groups.setdefault(prompt_family_for_run(item), []).append(item)
            prompt_families[subagent] = {}
            for family, family_runs in family_groups.items():
                fam_total = len(family_runs)
                fam_merge = sum(1 for item in family_runs if item.get("merge_ready") is True)
                fam_useful = sum(1 for item in family_runs if item.get("useful_progress") is True)
                prompt_families[subagent][family] = {
                    "runs": fam_total,
                    "merge_ready_rate": round(fam_merge / max(1, fam_total), 3),
                    "useful_progress_rate": round(fam_useful / max(1, fam_total), 3),
                }

        task_class_memory[task_class] = {
            "preferred_subagents": sorted(set(preferred)),
            "avoid_subagents": sorted(set(avoid)),
            "retry_useful_subagents": sorted(set(retry_useful)),
            "failure_prone_subagents": sorted(set(failure_prone)),
            "prompt_families": prompt_families,
        }

    recurring_failure_signatures: dict[str, Any] = {}
    for subagent, counts in subagent_signatures.items():
        recurring_failure_signatures[subagent] = [
            {"failure_reason": reason, "count": count}
            for reason, count in sorted(counts.items(), key=lambda item: (-item[1], item[0]))
            if count >= 2
        ]
    return task_class_memory, recurring_failure_signatures


def refresh_strategy(task_id: str) -> dict[str, Any]:
    snapshot = subagent_system.init_snapshot(task_id)
    config = vida_config.load_validated_config()
    routing = vida_config.dotted_get(config, "agent_system.routing", {}) or {}
    subagents = snapshot.get("subagents", {})
    scorecards = snapshot.get("scorecards", {})
    recent_runs = load_jsonl(RUN_LOG_PATH)[-300:]
    task_class_memory, recurring_failure_signatures = build_memory_hints(recent_runs)

    strategy = {
        "generated_at": now_utc(),
        "task_id": task_id,
        "subagents": {},
        "task_classes": {},
        "domains": {},
    }

    for subagent_name, subagent_cfg in subagents.items():
        scorecard = scorecards.get(subagent_name, {}).get("global", {})
        strategy["subagents"][subagent_name] = {
            "billing_tier": subagent_cfg.get("billing_tier", "unknown"),
            "speed_tier": subagent_cfg.get("speed_tier", "unknown"),
            "quality_tier": subagent_cfg.get("quality_tier", "unknown"),
            "write_scope": subagent_cfg.get("write_scope", "none"),
            "state": scorecard.get("state", "normal"),
            "lifecycle_stage": scorecard.get("lifecycle_stage", "declared"),
            "subagent_state": scorecard.get("subagent_state", "active"),
            "failure_reason": scorecard.get("failure_reason", ""),
            "cooldown_until": scorecard.get("cooldown_until", ""),
            "probe_required": bool(scorecard.get("probe_required", False)),
            "score": int(scorecard.get("score", 50)),
            "success_count": int(scorecard.get("success_count", 0)),
            "failure_count": int(scorecard.get("failure_count", 0)),
            "recovery_attempt_count": int(scorecard.get("recovery_attempt_count", 0)),
            "recovery_success_count": int(scorecard.get("recovery_success_count", 0)),
            "last_recovery_status": str(scorecard.get("last_recovery_status", "")),
            "useful_progress_count": int(scorecard.get("useful_progress_count", 0)),
            "chatter_only_count": int(scorecard.get("chatter_only_count", 0)),
            "useful_progress_rate": float(scorecard.get("useful_progress_rate", 0) or 0),
            "timeout_after_progress_count": int(scorecard.get("timeout_after_progress_count", 0)),
            "avg_time_to_first_useful_output_ms": int(scorecard.get("avg_time_to_first_useful_output_ms", 0) or 0),
            "budget_cost_units": subagent_system.subagent_budget_cost_units(subagent_cfg),
            "budget_efficiency_score": budget_efficiency_score(subagent_cfg, scorecard),
            "domains": scorecards.get(subagent_name, {}).get("by_domain", {}),
            "strengths": strengths_for_subagent(subagent_name, subagent_cfg, scorecard),
            "weaknesses": weaknesses_for_subagent(subagent_name, subagent_cfg, scorecard),
            "recurring_failure_signatures": recurring_failure_signatures.get(subagent_name, []),
        }

    discovered_domains: set[str] = set()
    for payload in scorecards.values():
        for domain_name in payload.get("by_domain", {}).keys():
            discovered_domains.add(str(domain_name))
    for domain_name in sorted(discovered_domains):
        ranked: list[dict[str, Any]] = []
        for subagent_name, payload in scorecards.items():
            domain_card = payload.get("by_domain", {}).get(domain_name)
            if not isinstance(domain_card, dict):
                continue
            ranked.append(
                {
                    "subagent": subagent_name,
                    "score": int(domain_card.get("score", 50)),
                    "state": str(domain_card.get("state", "normal")),
                }
            )
        ranked.sort(key=lambda item: (-item["score"], item["subagent"]))
        strategy["domains"][domain_name] = ranked

    for task_class in sorted(routing.keys()):
        route = subagent_system.route_subagent(task_class)
        ordered = [route.get("selected_subagent")] + [item.get("subagent") for item in route.get("fallback_subagents", [])]
        strategy["task_classes"][task_class] = {
            "recommended_order": [subagent for subagent in ordered if subagent],
            "fanout_subagents": route.get("fanout_subagents", []),
            "fanout_min_results": int(route.get("fanout_min_results", 0)),
            "merge_policy": route.get("merge_policy", "single_subagent"),
            "verification_gate": route.get("verification_gate"),
            "risk_class": route.get("risk_class", "R0"),
            "target_review_state": subagent_system.target_review_state_for(str(route.get("risk_class", "R0"))),
            "target_manifest_review_state": subagent_system.target_manifest_review_state_for(str(route.get("risk_class", "R0"))),
            "independent_verification_required": route.get("independent_verification_required", "no"),
            "verification_route_task_class": route.get("verification_route_task_class", ""),
            "verification_plan": route.get("verification_plan", {}),
            "route_budget": route.get("route_budget", {}),
            "route_graph": route.get("route_graph", {}),
            "memory_hints": task_class_memory.get(
                task_class,
                {
                    "preferred_subagents": [],
                    "avoid_subagents": [],
                    "retry_useful_subagents": [],
                    "failure_prone_subagents": [],
                    "prompt_families": {},
                },
            ),
        }

    save_json(STRATEGY_PATH, strategy)
    return strategy


def run(task_id: str) -> int:
    eval_pack = ensure_eval_pack(task_id)
    is_closed = task_closed(task_id)
    runs = [item for item in load_jsonl(RUN_LOG_PATH) if item.get("task_id") == task_id]
    processed = load_json(PROCESSED_PATH, {"processed_run_ids": []})
    processed_ids = set(processed.get("processed_run_ids", []))
    review_entries: list[dict[str, Any]] = []

    for run_item in runs:
        run_id = run_item.get("run_id")
        if not run_id or run_id in processed_ids:
            continue
        quality = quality_score_for(run_item, eval_pack, is_closed)
        result = "success" if quality >= 60 and run_item.get("exit_code") == 0 else "failure"
        domain_tags = infer_domain_tags(run_item)
        note = (
            f"task_closed={is_closed}; dispatch={run_item.get('dispatch_mode')}; "
            f"billing={run_item.get('billing_tier')}; output_bytes={run_item.get('output_bytes', 0)}; "
            f"merge_ready={run_item.get('merge_ready', False)}; "
            f"useful_progress={run_item.get('useful_progress', False)}; "
            f"chatter_only={run_item.get('chatter_only', False)}; "
            f"time_to_first_useful_output_ms={run_item.get('time_to_first_useful_output_ms')}; "
            f"verification_role={run_item.get('verification_role', '')}; "
            f"independent_verification_passed={run_item.get('independent_verification_passed', False)}; "
            f"independent_verification_failed={run_item.get('independent_verification_failed', False)}; "
            f"verification_caught_issue={run_item.get('verification_caught_issue', False)}; "
            f"subagent_state={run_item.get('subagent_state', 'active')}; "
            f"failure_reason={run_item.get('failure_reason', '')}; "
            f"cooldown_until={run_item.get('cooldown_until', '')}; "
            f"review_state={run_item.get('review_state', 'review_pending')}; "
            f"risk_class={run_item.get('risk_class', 'R0')}; "
            f"domains={','.join(domain_tags)}"
        )
        score_update = subagent_system.update_score(
            str(run_item.get("subagent")),
            result,
            str(run_item.get("task_class")),
            quality,
            int(run_item.get("duration_ms", 0) or 0),
            note,
            domain_tags,
            {
                "useful_progress": bool(run_item.get("useful_progress", False)),
                "chatter_only": bool(run_item.get("chatter_only", False)),
                "time_to_first_useful_output_ms": (
                    int(run_item.get("time_to_first_useful_output_ms", 0) or 0)
                    if run_item.get("time_to_first_useful_output_ms") is not None
                    else None
                ),
                "timeout_after_progress": (
                    str(run_item.get("status")) == "timeout"
                    and bool(run_item.get("useful_progress", False))
                ),
                "subagent_state": str(run_item.get("subagent_state", "active")),
                "failure_reason": str(run_item.get("failure_reason", "")),
                "cooldown_until": str(run_item.get("cooldown_until", "")),
                "probe_required": bool(run_item.get("probe_required", False)),
                "last_quota_exhausted_at": str(run_item.get("last_quota_exhausted_at", "")),
                "verification_role": str(run_item.get("verification_role", "")),
                "independent_verification_passed": bool(run_item.get("independent_verification_passed", False)),
                "independent_verification_failed": bool(run_item.get("independent_verification_failed", False)),
                "verification_caught_issue": bool(run_item.get("verification_caught_issue", False)),
            },
        )
        review_entries.append(
            {
                "run_id": run_id,
                "subagent": run_item.get("subagent"),
                "task_class": run_item.get("task_class"),
                "domain_tags": domain_tags,
                "quality_score": quality,
                "result": result,
                "duration_ms": int(run_item.get("duration_ms", 0) or 0),
                "risk_class": run_item.get("risk_class", "R0"),
                "review_state": run_item.get("review_state", "review_pending"),
                "merge_ready": bool(run_item.get("merge_ready", False)),
                "score_update": score_update,
            }
        )
        processed_ids.add(run_id)

    processed["processed_run_ids"] = sorted(processed_ids)
    save_json(PROCESSED_PATH, processed)
    strategy = refresh_strategy(task_id)
    review_payload = {
        "generated_at": now_utc(),
        "task_id": task_id,
        "task_closed": is_closed,
        "subagent_runs_seen": len(runs),
        "subagent_runs_processed": len(review_entries),
        "eval_pack": eval_pack,
        "review_entries": review_entries,
        "strategy_path": str(STRATEGY_PATH),
        "strategy_snapshot": strategy,
    }
    save_json(LOG_DIR / f"subagent-review-{task_id}.json", review_payload)
    return 0


def usage() -> int:
    print("Usage: python3 _vida/scripts/subagent-eval-pack.py run <task_id>", file=sys.stderr)
    return 1


def main(argv: list[str]) -> int:
    if len(argv) != 3 or argv[1] != "run":
        return usage()
    try:
        return run(argv[2])
    except (ValueError, vida_config.OverlayValidationError) as exc:
        print(f"[subagent-eval-pack] {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
