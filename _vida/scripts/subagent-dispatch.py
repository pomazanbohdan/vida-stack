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
RUN_LOG_PATH = LOG_DIR / "subagent-provider-runs.jsonl"


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


vida_config = load_module("vida_config_runtime_dispatch", SCRIPT_DIR / "vida-config.py")
subagent_system = load_module("subagent_system_runtime_dispatch", SCRIPT_DIR / "subagent-system.py")


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


def provider_selected_model(route: dict[str, Any], provider_name: str, provider_cfg: dict[str, Any]) -> str | None:
    if route.get("provider") == provider_name:
        return route.get("selected_model")
    for item in route.get("fallback_chain", []):
        if item.get("provider") == provider_name:
            return item.get("selected_model")
    default_model = provider_cfg.get("default_model")
    return default_model if isinstance(default_model, str) and default_model else None


def route_provider_item(route: dict[str, Any], provider_name: str) -> dict[str, Any]:
    if route.get("provider") == provider_name:
        return route
    for item in route.get("fallback_chain", []):
        if item.get("provider") == provider_name:
            return item
    return {}


def provider_command(
    provider_name: str,
    prompt: str,
    output_path: Path,
    workdir: Path,
    model: str | None,
    provider_cfg: dict[str, Any],
) -> tuple[list[str], bool]:
    dispatch_cfg = provider_cfg.get("dispatch", {})
    if not provider_supports_dispatch(provider_cfg):
        raise ValueError(f"Provider does not expose dispatch config: {provider_name}")

    command = str(dispatch_cfg.get("command", "")).strip()
    cmd = [command, *normalize_arg_list(dispatch_cfg.get("static_args"))]

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
            raise ValueError(f"Provider dispatch output_flag missing for file mode: {provider_name}")
        cmd.extend([output_flag, str(output_path)])
        use_stdout_output = False
    elif output_mode == "stdout":
        use_stdout_output = True
    else:
        raise ValueError(f"Unsupported provider dispatch output_mode={output_mode}: {provider_name}")

    prompt_mode = str(dispatch_cfg.get("prompt_mode", "positional")).strip() or "positional"
    if prompt_mode == "flag":
        prompt_flag = str(dispatch_cfg.get("prompt_flag", "")).strip()
        if not prompt_flag:
            raise ValueError(f"Provider dispatch prompt_flag missing for flag mode: {provider_name}")
        cmd.extend([prompt_flag, prompt])
    elif prompt_mode == "positional":
        cmd.append(prompt)
    else:
        raise ValueError(f"Unsupported provider dispatch prompt_mode={prompt_mode}: {provider_name}")
    return cmd, use_stdout_output


def provider_env(provider_cfg: dict[str, Any]) -> dict[str, str]:
    dispatch_cfg = provider_cfg.get("dispatch", {})
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


def provider_supports_dispatch(provider_cfg: dict[str, Any]) -> bool:
    dispatch_cfg = provider_cfg.get("dispatch", {})
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


def route_runtime_limit(route: dict[str, Any], provider_cfg: dict[str, Any]) -> int:
    route_limit = policy_int(route.get("max_runtime_seconds"), 0)
    provider_limit = policy_int(provider_cfg.get("max_runtime_seconds"), 0)
    return route_limit or provider_limit or 180


def dispatch_runtime_limit(
    base_limit: int,
    dispatch_mode: str,
    provider_route: dict[str, Any],
    provider_cfg: dict[str, Any],
) -> int:
    limit = max(60, base_limit)
    orchestration_tier = policy_value(provider_route.get("orchestration_tier"), policy_value(provider_cfg.get("orchestration_tier"), "standard"))
    quality_tier = policy_value(provider_cfg.get("quality_tier"), "medium")
    if dispatch_mode == "fallback":
        if orchestration_tier == "bridge":
            limit = max(limit, 240)
        if quality_tier == "high":
            limit = max(limit, 220)
    if dispatch_mode == "arbitration":
        limit = min(limit, 180)
    return limit


def route_min_output_bytes(route: dict[str, Any], provider_cfg: dict[str, Any]) -> int:
    route_min = policy_int(route.get("min_output_bytes"), 0)
    provider_min = policy_int(provider_cfg.get("min_output_bytes"), 0)
    return route_min or provider_min or 220


def route_risk_class(route: dict[str, Any]) -> str:
    value = str(route.get("risk_class", "R0")).strip().upper()
    return value if value in {"R0", "R1", "R2", "R3", "R4"} else "R0"


def review_state_for(status: str, merge_ready: bool, risk_class: str) -> str:
    if status != "success":
        return "review_failed"
    if not merge_ready:
        return "review_pending"
    if risk_class == "R0":
        return "promotion_ready"
    if risk_class == "R1":
        return "policy_check_pending"
    if risk_class == "R2":
        return "senior_review_pending"
    return "requires_human"


def manifest_review_state(summary: dict[str, Any], risk_class: str) -> str:
    if summary.get("provider_exhausted") and not summary.get("decision_ready"):
        return "review_failed"
    if summary.get("tie_break_recommended") or summary.get("open_conflicts"):
        return "review_pending"
    if risk_class == "R0":
        return "promotion_ready"
    if risk_class == "R1":
        return "policy_check_pending"
    if risk_class == "R2":
        return "senior_review_pending"
    return "requires_human"


def infer_domain_tags(prompt: str, task_class: str) -> list[str]:
    text = f"{task_class}\n{prompt}".casefold()
    tags: list[str] = []
    if any(token in text for token in ["api", "json", "schema", "payload", "endpoint"]):
        tags.append("api_contract")
    if any(token in text for token in ["auth", "session", "token", "bearer", "security"]):
        tags.append("auth_security")
    if any(token in text for token in ["ui", "widget", "layout", "render", "component"]):
        tags.append("frontend_ui")
    if any(token in text for token in ["state", "store", "provider", "cache", "repository"]):
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
    if normalized.startswith("let me ") or normalized.startswith("now let me "):
        return False
    return any(
        token in normalized
        for token in [
            "## findings",
            "root cause",
            "recommended",
            "severity",
            "evidence",
            "\n- ",
            "\n1. ",
        ]
    )


def output_has_useful_progress(output_text: str, stderr_text: str, min_output_bytes: int) -> bool:
    combined = f"{output_text}\n{stderr_text}".strip()
    if not combined:
        return False
    normalized = combined.casefold()
    useful_markers = [
        "## findings",
        "root cause",
        "severity",
        "evidence",
        "confirmed",
        "location:",
        "read ",
        "grep ",
        "glob ",
        "file:",
        "path:",
        "docs/",
    ]
    if any(marker in normalized for marker in useful_markers):
        return True
    if len(output_text.strip().encode("utf-8")) >= max(160, min_output_bytes // 2):
        return True
    return False


def write_manifest(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def provider_result_payload(
    *,
    task_id: str,
    task_class: str,
    provider_name: str,
    selected_model: str | None,
    provider_cfg: dict[str, Any],
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
    output_text = load_output_text(output_file)
    stderr_text = load_output_text(stderr_path)
    merge_ready = status == "success" and output_is_merge_ready(output_text, min_output_bytes)
    useful_progress = output_has_useful_progress(output_text, stderr_text, min_output_bytes)
    review_state = review_state_for(status, merge_ready, risk_class)
    payload = {
        "ts": now_utc(),
        "type": "subagent_provider_run",
        "run_id": run_id,
        "task_id": task_id,
        "task_class": task_class,
        "dispatch_mode": dispatch_mode,
        "provider": provider_name,
        "selected_model": selected_model,
        "billing_tier": provider_cfg.get("billing_tier", "unknown"),
        "speed_tier": provider_cfg.get("speed_tier", "unknown"),
        "quality_tier": provider_cfg.get("quality_tier", "unknown"),
        "specialties": provider_cfg.get("specialties", []),
        "write_scope": provider_cfg.get("write_scope", "none"),
        "ts_start": ts_start,
        "ts_end": now_utc(),
        "duration_ms": duration_ms,
        "exit_code": exit_code,
        "status": status,
        "merge_ready": merge_ready,
        "useful_progress": useful_progress,
        "review_state": review_state,
        "risk_class": risk_class,
        "domain_tags": domain_tags,
        "max_runtime_seconds": max_runtime_seconds,
        "min_output_bytes": min_output_bytes,
        "output_file": str(output_file),
        "stderr_file": str(stderr_path),
        "output_bytes": output_size(output_file),
        "stderr_bytes": output_size(stderr_path),
        "time_to_first_useful_output_ms": launch_time_to_first_useful_output_ms(started, output_file, stderr_path, min_output_bytes),
        "workdir": str(workdir),
        "prompt_file": str(prompt_file),
        "route_provider": route.get("provider"),
        "verification_gate": route.get("verification_gate"),
        "merge_policy": route.get("merge_policy"),
        "error": error_text,
    }
    append_jsonl(RUN_LOG_PATH, payload)
    return payload


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
    return {
        "output_bytes": output_bytes,
        "stderr_bytes": stderr_bytes,
        "useful_progress": useful_progress,
        "grew": grew,
        "first_useful_output_ms": launch.get("first_useful_output_ms"),
        "idle_seconds": int(now_mono - float(launch.get("last_activity_at", launch["started"]))),
    }


def runtime_extension_seconds(launch: dict[str, Any]) -> int:
    base_limit = int(launch["max_runtime_seconds"])
    quality_tier = str(launch["provider_cfg"].get("quality_tier", "medium"))
    if quality_tier == "high":
        return min(90, max(30, base_limit // 2))
    return min(60, max(20, base_limit // 3))


def run_provider(
    task_id: str,
    task_class: str,
    provider_name: str,
    prompt_file: Path,
    output_file: Path,
    workdir: Path,
    route: dict[str, Any],
    provider_cfg: dict[str, Any],
    dispatch_mode: str,
) -> dict[str, Any]:
    ensure_dirs()
    output_file.parent.mkdir(parents=True, exist_ok=True)
    stderr_path = output_file.with_suffix(output_file.suffix + ".stderr.log")
    prompt = read_prompt(prompt_file)
    selected_model = provider_selected_model(route, provider_name, provider_cfg)
    provider_route = route_provider_item(route, provider_name)
    domain_tags = infer_domain_tags(prompt, task_class)
    risk_class = route_risk_class(route)
    max_runtime_seconds = dispatch_runtime_limit(
        route_runtime_limit(provider_route or route, provider_cfg),
        dispatch_mode,
        provider_route or route,
        provider_cfg,
    )
    min_output_bytes = route_min_output_bytes(route, provider_cfg)
    run_id = f"spr-{uuid.uuid4().hex[:12]}"
    ts_start = now_utc()
    started = time.monotonic()
    if not provider_supports_dispatch(provider_cfg):
        error_text = f"provider dispatch unavailable for {provider_name}; internal/senior lanes require orchestrator-owned handling"
        stderr_path.write_text(error_text + "\n", encoding="utf-8")
        return provider_result_payload(
            task_id=task_id,
            task_class=task_class,
            provider_name=provider_name,
            selected_model=selected_model,
            provider_cfg=provider_cfg,
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
    cmd, use_stdout_output = provider_command(provider_name, prompt, output_file, workdir, selected_model, provider_cfg)
    exit_code = 0
    status = "success"
    error_text = ""

    env = os.environ.copy()
    env.update(provider_env(provider_cfg))
    try:
        with stderr_path.open("w", encoding="utf-8") as stderr_handle:
            if use_stdout_output:
                with output_file.open("w", encoding="utf-8") as stdout_handle:
                    completed = subprocess.run(
                        cmd,
                        cwd=str(workdir),
                        stdout=stdout_handle,
                        stderr=stderr_handle,
                        text=True,
                        check=False,
                        env=env,
                        timeout=max_runtime_seconds,
                    )
            else:
                completed = subprocess.run(
                    cmd,
                    cwd=str(workdir),
                    stdout=stderr_handle,
                    stderr=stderr_handle,
                    text=True,
                    check=False,
                    env=env,
                    timeout=max_runtime_seconds,
                )
        exit_code = int(completed.returncode)
        if exit_code != 0 or output_size(output_file) == 0:
            status = "failure"
    except subprocess.TimeoutExpired:
        exit_code = 124
        status = "timeout"
        error_text = f"provider exceeded runtime limit ({max_runtime_seconds}s)"
        stderr_path.write_text(error_text + "\n", encoding="utf-8")
    except Exception as exc:
        exit_code = 1
        status = "failure"
        error_text = str(exc)
        stderr_path.write_text(error_text + "\n", encoding="utf-8")

    return provider_result_payload(
        task_id=task_id,
        task_class=task_class,
        provider_name=provider_name,
        selected_model=selected_model,
        provider_cfg=provider_cfg,
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
        status=status,
        exit_code=exit_code,
        error_text=error_text,
    )


def start_provider_process(
    task_id: str,
    task_class: str,
    provider_name: str,
    prompt_file: Path,
    output_file: Path,
    workdir: Path,
    route: dict[str, Any],
    provider_cfg: dict[str, Any],
    dispatch_mode: str,
) -> dict[str, Any]:
    ensure_dirs()
    output_file.parent.mkdir(parents=True, exist_ok=True)
    stderr_path = output_file.with_suffix(output_file.suffix + ".stderr.log")
    prompt = read_prompt(prompt_file)
    selected_model = provider_selected_model(route, provider_name, provider_cfg)
    provider_route = route_provider_item(route, provider_name)
    domain_tags = infer_domain_tags(prompt, task_class)
    risk_class = route_risk_class(route)
    max_runtime_seconds = dispatch_runtime_limit(
        route_runtime_limit(provider_route or route, provider_cfg),
        dispatch_mode,
        provider_route or route,
        provider_cfg,
    )
    min_output_bytes = route_min_output_bytes(route, provider_cfg)
    run_id = f"spr-{uuid.uuid4().hex[:12]}"
    ts_start = now_utc()
    started = time.monotonic()

    if not provider_supports_dispatch(provider_cfg):
        error_text = f"provider dispatch unavailable for {provider_name}; internal/senior lanes require orchestrator-owned handling"
        stderr_path.write_text(error_text + "\n", encoding="utf-8")
        return {
            "result": provider_result_payload(
                task_id=task_id,
                task_class=task_class,
                provider_name=provider_name,
                selected_model=selected_model,
                provider_cfg=provider_cfg,
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
        cmd, use_stdout_output = provider_command(provider_name, prompt, output_file, workdir, selected_model, provider_cfg)
        stderr_handle = stderr_path.open("w", encoding="utf-8")
        stdout_handle = output_file.open("w", encoding="utf-8") if use_stdout_output else None
        process = subprocess.Popen(
            cmd,
            cwd=str(workdir),
            stdout=stdout_handle if stdout_handle is not None else stderr_handle,
            stderr=stderr_handle,
            text=True,
            env={**os.environ.copy(), **provider_env(provider_cfg)},
        )
        return {
            "process": process,
            "stdout_handle": stdout_handle,
            "stderr_handle": stderr_handle,
            "task_id": task_id,
            "task_class": task_class,
            "provider_name": provider_name,
            "prompt_file": prompt_file,
            "output_file": output_file,
            "stderr_path": stderr_path,
            "workdir": workdir,
            "route": route,
            "provider_cfg": provider_cfg,
            "dispatch_mode": dispatch_mode,
            "selected_model": selected_model,
            "domain_tags": domain_tags,
            "risk_class": risk_class,
            "max_runtime_seconds": max_runtime_seconds,
            "min_output_bytes": min_output_bytes,
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
            "result": provider_result_payload(
                task_id=task_id,
                task_class=task_class,
                provider_name=provider_name,
                selected_model=selected_model,
                provider_cfg=provider_cfg,
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


def finalize_provider_process(
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
    return provider_result_payload(
        task_id=launch["task_id"],
        task_class=launch["task_class"],
        provider_name=launch["provider_name"],
        selected_model=launch["selected_model"],
        provider_cfg=launch["provider_cfg"],
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


def terminate_provider_process(
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
    return finalize_provider_process(
        launch,
        status_override=status_override,
        exit_code_override=exit_code_override,
        error_text=reason,
    )


def route_snapshot(task_class: str, task_id: str | None = None) -> tuple[dict[str, Any], dict[str, Any]]:
    snapshot = subagent_system.init_snapshot(task_id)
    route = subagent_system.route_provider(task_class)
    return snapshot, route


def candidate_provider_cfg(snapshot: dict[str, Any], provider_name: str) -> dict[str, Any]:
    providers = snapshot.get("providers", {})
    cfg = providers.get(provider_name, {})
    if not cfg:
        raise ValueError(f"Provider not found in snapshot: {provider_name}")
    return cfg


def run_single(argv: list[str]) -> int:
    if len(argv) < 7:
        print(
            "Usage: python3 _vida/scripts/subagent-dispatch.py provider <task_id> <task_class> <provider> <prompt_file> <output_file> [workdir]",
            file=sys.stderr,
        )
        return 1
    task_id, task_class, provider_name, prompt_file_raw, output_file_raw = argv[2:7]
    workdir = Path(argv[7]).resolve() if len(argv) > 7 else ROOT_DIR
    prompt_file = Path(prompt_file_raw).resolve()
    output_file = Path(output_file_raw).resolve()
    snapshot, route = route_snapshot(task_class, task_id)
    payload = run_provider(
        task_id,
        task_class,
        provider_name,
        prompt_file,
        output_file,
        workdir,
        route,
        candidate_provider_cfg(snapshot, provider_name),
        "single",
    )
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0 if payload["status"] == "success" else 2


def ensemble_providers(route: dict[str, Any]) -> list[str]:
    providers = list(route.get("fanout_providers", []))
    primary = route.get("provider")
    if primary and primary not in providers:
        providers.insert(0, primary)
    if not providers and primary:
        providers = [primary]
    deduped: list[str] = []
    seen: set[str] = set()
    for provider in providers:
        if provider in seen:
            continue
        deduped.append(provider)
        seen.add(provider)
    return deduped


def load_output_text(path: Path) -> str:
    if not path.exists():
        return ""
    try:
        return path.read_text(encoding="utf-8").strip()
    except OSError:
        return ""


def digest_text(text: str) -> str:
    if not text:
        return ""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


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


def preview_text(text: str, limit: int = 160) -> str:
    collapsed = re.sub(r"\s+", " ", text).strip()
    if len(collapsed) <= limit:
        return collapsed
    return collapsed[: limit - 3] + "..."


def cluster_payload(group_key: str, providers: list[str], preview: str, weight: int = 0) -> dict[str, Any]:
    return {
        "cluster_id": group_key[:12] if group_key else "empty",
        "providers": sorted(providers),
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
    provider_scores: dict[str, int] | None = None,
) -> dict[str, Any]:
    success_items = [item for item in results if item.get("status") == "success" and item.get("merge_ready") is True]
    failure_providers = sorted(item["provider"] for item in results if item.get("status") != "success")
    non_merge_ready_providers = sorted(
        item["provider"]
        for item in results
        if item.get("status") == "success" and item.get("merge_ready") is not True
    )
    required_results = max(1, min_results)
    provider_scores = provider_scores or {}
    exact_groups: dict[str, list[str]] = {}
    semantic_groups: dict[str, list[str]] = {}
    semantic_previews: dict[str, str] = {}
    provider_evidence_weights: dict[str, int] = {}
    for item in success_items:
        text = load_output_text(Path(item["output_file"]))
        provider_evidence_weights[item["provider"]] = source_backing_weight(text)
        exact_text = text if text else f"empty:{item['provider']}"
        exact_digest = digest_text(exact_text)
        exact_groups.setdefault(exact_digest, []).append(item["provider"])

        semantic_text = normalize_output_text(text)
        if not semantic_text:
            semantic_text = f"empty:{item['provider']}"
        semantic_digest = digest_text(semantic_text)
        semantic_groups.setdefault(semantic_digest, []).append(item["provider"])
        semantic_previews.setdefault(semantic_digest, preview_text(semantic_text))

    exact_agreements = sorted(sorted(group) for group in exact_groups.values() if len(group) > 1)
    semantic_weight_index = {
        group_key: sum(
            int(provider_scores.get(provider, 0)) + int(provider_evidence_weights.get(provider, 0))
            for provider in providers
        )
        for group_key, providers in semantic_groups.items()
    }
    semantic_clusters = sorted(
        (
            cluster_payload(
                group_key,
                providers,
                semantic_previews.get(group_key, ""),
                semantic_weight_index.get(group_key, 0),
            )
            for group_key, providers in semantic_groups.items()
        ),
        key=lambda item: (
            -len(item["providers"]),
            -int(item.get("weight", 0)),
            item["providers"],
        ),
    )
    semantic_agreements = [cluster for cluster in semantic_clusters if len(cluster["providers"]) > 1]
    unique_findings = [cluster for cluster in semantic_clusters if len(cluster["providers"]) == 1]
    cluster_weights = {cluster["cluster_id"]: int(cluster.get("weight", 0)) for cluster in semantic_clusters}

    exact_consensus = len(success_items) > 1 and len(exact_groups) == 1
    semantic_consensus = len(success_items) > 1 and len(semantic_groups) == 1
    largest_cluster = semantic_clusters[0] if semantic_clusters else None
    second_cluster_size = len(semantic_clusters[1]["providers"]) if len(semantic_clusters) > 1 else 0
    dominant_weight = cluster_weights.get(largest_cluster["cluster_id"], 0) if largest_cluster else 0
    second_weight = cluster_weights.get(semantic_clusters[1]["cluster_id"], 0) if len(semantic_clusters) > 1 else 0
    semantic_majority = (
        largest_cluster is not None
        and len(largest_cluster["providers"]) >= required_results
        and len(largest_cluster["providers"]) > second_cluster_size
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
        "success_providers": sorted(item["provider"] for item in success_items),
        "failure_providers": failure_providers,
        "non_merge_ready_providers": non_merge_ready_providers,
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
        "provider_evidence_weights": provider_evidence_weights,
        "tie_break_recommended": tie_break_recommended,
        "tie_break_reason": tie_break_reason,
        "provider_exhausted": len(success_items) < required_results,
        "orchestrator_review_required": tie_break_recommended or len(open_conflicts) > 0,
    }


def clone_json_payload(payload: Any) -> Any:
    return json.loads(json.dumps(payload))


def route_provider_scores(route: dict[str, Any]) -> dict[str, int]:
    scores: dict[str, int] = {}
    primary = route.get("provider")
    if isinstance(primary, str) and primary:
        scores[primary] = int(route.get("effective_score", 0))
    for item in route.get("fallback_chain", []):
        provider_name = item.get("provider")
        if not isinstance(provider_name, str) or not provider_name or provider_name in scores:
            continue
        scores[provider_name] = int(item.get("effective_score", 0))
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
    ordered_providers: list[str] = []
    for provider_name in [route.get("provider"), *requested_fanout]:
        if isinstance(provider_name, str) and provider_name and provider_name not in ordered_providers:
            ordered_providers.append(provider_name)
    for item in route.get("fallback_chain", []):
        provider_name = item.get("provider")
        if isinstance(provider_name, str) and provider_name and provider_name not in ordered_providers:
            ordered_providers.append(provider_name)

    scores = route_provider_scores(route)
    result_by_provider = {item["provider"]: item for item in results}
    providers = snapshot.get("providers", {})
    candidates: list[dict[str, Any]] = []
    for provider_name in ordered_providers:
        provider_cfg = providers.get(provider_name, {})
        if not provider_supports_dispatch(provider_cfg):
            continue
        if not provider_cfg.get("enabled") or not provider_cfg.get("available"):
            continue
        prior = result_by_provider.get(provider_name)
        if prior and prior.get("status") != "success":
            continue
        used = prior is not None
        candidates.append(
            {
                "provider": provider_name,
                "used": used,
                "selection_reason": "rerun_best_available_provider" if used else "unused_supported_provider",
                "quality_rank": quality_rank(provider_cfg.get("quality_tier")),
                "role_rank": role_rank(provider_cfg.get("role")),
                "effective_score": scores.get(provider_name, 0),
            }
        )

    candidates.sort(
        key=lambda item: (
            1 if item["used"] else 0,
            -int(item["quality_rank"]),
            -int(item["role_rank"]),
            -int(item["effective_score"]),
            str(item["provider"]),
        )
    )
    return candidates


def arbitration_prompt_text(
    original_prompt: str,
    task_class: str,
    merge_summary: dict[str, Any],
    results: list[dict[str, Any]],
) -> str:
    result_by_provider = {item["provider"]: item for item in results if item.get("status") == "success"}
    allowed_cluster_ids = [cluster.get("cluster_id", "") for cluster in merge_summary.get("open_conflicts", [])]
    lines = [
        "You are the bounded arbitration lane for VIDA ensemble conflict resolution.",
        "Select one existing semantic cluster or return no_decision.",
        "Do not propose a new answer, do not merge clusters, and do not expand scope.",
        f"Task class: {task_class}",
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
        providers = [str(provider) for provider in cluster.get("providers", []) if isinstance(provider, str)]
        sample = str(cluster.get("sample", "")).strip()
        lines.append(f"- cluster_id: {cluster_id}")
        lines.append(f"  providers: {', '.join(providers) if providers else '(none)'}")
        lines.append(f"  normalized_sample: {sample or '(empty)'}")
        for provider_name in providers:
            payload = result_by_provider.get(provider_name)
            if not payload:
                continue
            raw_excerpt = preview_text(load_output_text(Path(payload["output_file"])), 420)
            if raw_excerpt:
                lines.append(f"  {provider_name}_excerpt: {raw_excerpt}")
        lines.append("")
    lines.extend(
        [
            "Return ONLY valid JSON with this shape:",
            '{"decision":"select_cluster|no_decision","selected_cluster_id":"<allowed id or empty>","confidence":"high|medium|low","rationale":"<short reason>"}',
            f"Allowed cluster_ids: {', '.join(cluster_id for cluster_id in allowed_cluster_ids if cluster_id)}",
        ]
    )
    return "\n".join(lines).strip() + "\n"


def parse_json_object(text: str) -> dict[str, Any]:
    stripped = text.strip()
    candidates = [stripped]
    start = stripped.find("{")
    end = stripped.rfind("}")
    if start != -1 and end != -1 and end > start:
        candidates.append(stripped[start : end + 1])
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


def apply_arbitration_decision(
    merge_summary: dict[str, Any],
    arbitration_decision: dict[str, Any],
    arbitration_provider: str,
) -> dict[str, Any]:
    post_summary = clone_json_payload(merge_summary)
    post_summary["arbitrated_consensus"] = False
    post_summary["arbitration_provider"] = arbitration_provider
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
        "selected_provider": None,
        "selection_reason": "",
        "provider_reused": False,
        "candidate_providers": [],
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
    arbitration["candidate_providers"] = [item["provider"] for item in candidates]
    if not candidates:
        arbitration["status"] = "unavailable"
        arbitration["selection_reason"] = "no_supported_arbitration_provider"
        return arbitration, unresolved_arbitration_summary(merge_summary, "arbitration_provider_unavailable")

    selected = candidates[0]
    provider_name = str(selected["provider"])
    arbitration["selected_provider"] = provider_name
    arbitration["selection_reason"] = str(selected["selection_reason"])
    arbitration["provider_reused"] = bool(selected["used"])

    arbitration_prompt_file = output_dir / "arbitration-prompt.txt"
    arbitration_output_file = output_dir / f"{provider_name}.arbitration.txt"
    arbitration_prompt_file.write_text(
        arbitration_prompt_text(read_prompt(prompt_file), task_class, merge_summary, results),
        encoding="utf-8",
    )
    arbitration["prompt_file"] = str(arbitration_prompt_file)
    arbitration["output_file"] = str(arbitration_output_file)

    payload = run_provider(
        task_id,
        task_class,
        provider_name,
        arbitration_prompt_file,
        arbitration_output_file,
        workdir,
        route,
        candidate_provider_cfg(snapshot, provider_name),
        "arbitration",
    )
    arbitration["run"] = payload
    arbitration["status"] = str(payload.get("status", "failure"))

    if payload.get("status") != "success":
        return arbitration, unresolved_arbitration_summary(merge_summary, "arbitration_provider_failed")

    allowed_cluster_ids = [str(cluster.get("cluster_id", "")) for cluster in merge_summary.get("open_conflicts", [])]
    arbitration["decision"] = parse_arbitration_decision(load_output_text(arbitration_output_file), allowed_cluster_ids)
    return arbitration, apply_arbitration_decision(merge_summary, arbitration["decision"], provider_name)


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
    requested_fanout = ensemble_providers(route)
    max_parallel_agents = max(1, int(snapshot.get("agent_system", {}).get("max_parallel_agents", 1)))
    primary_fanout = requested_fanout[:max_parallel_agents]
    min_results = int(route.get("fanout_min_results", 0))
    required_results = max(1, min_results)
    manifest_path = output_dir / "manifest.json"

    results: list[dict[str, Any]] = []
    manifest: dict[str, Any] = {
        "generated_at": now_utc(),
        "task_id": task_id,
        "task_class": task_class,
        "workdir": str(workdir),
        "route": route,
        "requested_fanout_providers": requested_fanout,
        "fanout_providers": primary_fanout,
        "fanout_min_results": min_results,
        "max_parallel_agents": max_parallel_agents,
        "risk_class": route_risk_class(route),
        "review_state": "review_pending",
        "success_count": 0,
        "useful_progress_count": 0,
        "provider_exhausted": False,
        "fallback_used": False,
        "merge_summary": {},
        "arbitration": {},
        "post_arbitration_merge_summary": {},
        "results": [],
        "status": "running",
        "phase": "fanout_running",
    }
    write_manifest(manifest_path, manifest)
    launches: dict[str, dict[str, Any]] = {}
    for provider_name in primary_fanout:
        launch = start_provider_process(
            task_id,
            task_class,
            provider_name,
            prompt_file,
            output_dir / f"{provider_name}.txt",
            workdir,
            route,
            candidate_provider_cfg(snapshot, provider_name),
            "fanout",
        )
        if "result" in launch:
            results.append(launch["result"])
        else:
            launches[provider_name] = launch

    manifest["results"] = sorted(results, key=lambda item: item["provider"])
    manifest["success_count"] = sum(
        1 for item in results if item.get("status") == "success" and item.get("merge_ready") is True
    )
    write_manifest(manifest_path, manifest)

    while launches:
        completed_providers: list[str] = []
        loop_progress = False
        for provider_name, launch in list(launches.items()):
            process: subprocess.Popen[str] = launch["process"]
            progress = launch_progress_snapshot(launch)
            elapsed = time.monotonic() - float(launch["started"])
            effective_runtime_seconds = int(launch.get("effective_runtime_seconds", launch["max_runtime_seconds"]))
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
                    terminate_provider_process(
                        launch,
                        f"provider exceeded runtime limit ({launch.get('effective_runtime_seconds', launch['max_runtime_seconds'])}s)",
                        status_override="timeout",
                        exit_code_override=124,
                    )
                )
                completed_providers.append(provider_name)
                loop_progress = True
                continue
            if process.poll() is not None:
                results.append(finalize_provider_process(launch))
                completed_providers.append(provider_name)
                loop_progress = True

        for provider_name in completed_providers:
            launches.pop(provider_name, None)

        success_count = sum(
            1 for item in results if item.get("status") == "success" and item.get("merge_ready") is True
        )
        manifest["results"] = sorted(results, key=lambda item: item["provider"])
        manifest["success_count"] = success_count
        manifest["useful_progress_count"] = sum(1 for launch in launches.values() if launch.get("useful_progress"))
        write_manifest(manifest_path, manifest)

        if success_count >= required_results and launches:
            for provider_name, launch in list(launches.items()):
                results.append(
                    terminate_provider_process(
                        launch,
                        "terminated after required merge-ready ensemble results were reached",
                    )
                )
                launches.pop(provider_name, None)
            manifest["results"] = sorted(results, key=lambda item: item["provider"])
            manifest["success_count"] = success_count
            manifest["useful_progress_count"] = 0
            write_manifest(manifest_path, manifest)
            break

        max_possible_successes = success_count + len(launches)
        if max_possible_successes < required_results and launches:
            for provider_name, launch in list(launches.items()):
                results.append(
                    terminate_provider_process(
                        launch,
                        "terminated because fanout_min_results became unreachable with remaining providers",
                    )
                )
                launches.pop(provider_name, None)
            manifest["results"] = sorted(results, key=lambda item: item["provider"])
            manifest["success_count"] = success_count
            manifest["useful_progress_count"] = 0
            manifest["provider_exhausted"] = True
            write_manifest(manifest_path, manifest)
            break

        if not loop_progress and launches:
            time.sleep(0.5)

    success_count = sum(
        1 for item in results if item.get("status") == "success" and item.get("merge_ready") is True
    )
    fallback_used = False
    if success_count < min_results:
        manifest["phase"] = "fallback_running"
        write_manifest(manifest_path, manifest)
        for item in route.get("fallback_chain", []):
            provider_name = item.get("provider")
            if not provider_name or provider_name in primary_fanout:
                continue
            provider_cfg = candidate_provider_cfg(snapshot, provider_name)
            if not provider_supports_dispatch(provider_cfg):
                continue
            fallback_used = True
            result = run_provider(
                task_id,
                task_class,
                provider_name,
                prompt_file,
                output_dir / f"{provider_name}.txt",
                workdir,
                route,
                provider_cfg,
                "fallback",
            )
            results.append(result)
            manifest["results"] = sorted(results, key=lambda item: item["provider"])
            if result.get("status") == "success" and result.get("merge_ready") is True:
                success_count += 1
            manifest["success_count"] = success_count
            manifest["fallback_used"] = True
            write_manifest(manifest_path, manifest)
            if success_count >= min_results:
                break

    manifest["phase"] = "merge_evaluating"
    write_manifest(manifest_path, manifest)
    merge_summary = build_merge_summary(
        results,
        str(route.get("merge_policy", "single_provider")),
        min_results,
        provider_scores=route_provider_scores(route),
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
        "provider_exhausted": success_count < required_results,
        "fallback_used": fallback_used,
        "merge_summary": merge_summary,
        "arbitration": arbitration,
        "post_arbitration_merge_summary": post_arbitration_merge_summary,
        "results": sorted(results, key=lambda item: item["provider"]),
        "review_state": manifest_review_state(
            post_arbitration_merge_summary or merge_summary,
            route_risk_class(route),
        ),
        "status": (
            "completed"
            if (
                success_count >= required_results
                or (post_arbitration_merge_summary or merge_summary).get("decision_ready")
            )
            else "provider_exhausted"
        ),
        "phase": (
            "completed"
            if (
                success_count >= required_results
                or (post_arbitration_merge_summary or merge_summary).get("decision_ready")
            )
            else "provider_exhausted"
        ),
    }
    write_manifest(manifest_path, manifest)
    print(str(manifest_path))
    return 0 if (success_count >= required_results or (post_arbitration_merge_summary or merge_summary).get("decision_ready")) else 2


def usage() -> int:
    print(
        "Usage:\n"
        "  python3 _vida/scripts/subagent-dispatch.py provider <task_id> <task_class> <provider> <prompt_file> <output_file> [workdir]\n"
        "  python3 _vida/scripts/subagent-dispatch.py ensemble <task_id> <task_class> <prompt_file> <output_dir> [workdir]",
        file=sys.stderr,
    )
    return 1


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        return usage()
    cmd = argv[1]
    if cmd == "provider":
        return run_single(argv)
    if cmd == "ensemble":
        return run_ensemble(argv)
    return usage()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
