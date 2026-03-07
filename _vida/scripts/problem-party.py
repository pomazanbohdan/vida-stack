#!/usr/bin/env python3
"""Bounded multi-role discussion helper for VIDA problem-party protocol."""

from __future__ import annotations

import argparse
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT_DIR = Path(__file__).resolve().parents[2]
LOG_DIR = ROOT_DIR / ".vida" / "logs" / "problem-party"

BOARD_PRESETS: dict[str, dict[str, Any]] = {
    "small": {
        "default_rounds": 1,
        "max_rounds": 2,
        "roles": [
            {"id": "architect", "focus": "system shape, protocol cleanliness, boundary discipline"},
            {"id": "runtime_systems", "focus": "runtime gates, failure modes, operability"},
            {"id": "quality_verification", "focus": "proof, regressions, verification and residual risk"},
            {"id": "delivery_cost", "focus": "cost, token efficiency, implementation leverage"},
        ],
    },
    "large": {
        "default_rounds": 2,
        "max_rounds": 3,
        "roles": [
            {"id": "architect", "focus": "system shape, protocol cleanliness, boundary discipline"},
            {"id": "runtime_systems", "focus": "runtime gates, failure modes, operability"},
            {"id": "quality_verification", "focus": "proof, regressions, verification and residual risk"},
            {"id": "delivery_cost", "focus": "cost, token efficiency, implementation leverage"},
            {"id": "product_scope", "focus": "scope framing, expected behavior, user-visible semantics"},
            {"id": "security_safety", "focus": "safety, abuse paths, permission and risk posture"},
            {"id": "sre_observability", "focus": "telemetry, health signals, live diagnostics"},
            {"id": "data_contracts", "focus": "contracts, equivalence, schema and artifact fidelity"},
            {"id": "dx_tooling", "focus": "tool ergonomics, maintainability, developer friction"},
            {"id": "pm_process", "focus": "workflow sequencing, queue hygiene, closure criteria"},
        ],
    },
}


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def safe_slug(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "-", value.strip()).strip("-") or "topic"


def load_json(path: Path, default: Any) -> Any:
    if not path.exists():
        return default
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return default


def write_json(path: Path, payload: Any) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def render_board(
    task_id: str,
    topic: str,
    *,
    board: str,
    rounds: int | None,
    problem_payload: dict[str, Any],
    output_dir: Path | None,
) -> Path:
    preset = BOARD_PRESETS[board]
    round_count = rounds if rounds is not None else int(preset["default_rounds"])
    round_count = max(1, min(round_count, int(preset["max_rounds"])))
    effective_output_dir = output_dir or LOG_DIR
    manifest_path = effective_output_dir / f"{safe_slug(task_id)}.{safe_slug(topic)}.board.json"
    roles = []
    for item in preset["roles"]:
        role_id = str(item["id"])
        focus = str(item["focus"])
        roles.append(
            {
                "role": role_id,
                "focus": focus,
                "prompt": (
                    f"Role={role_id}. Focus on {focus}. "
                    "Return one bounded position, top risks, preferred option, and why not the main alternatives."
                ),
            }
        )
    payload = {
        "generated_at": now_utc(),
        "task_id": task_id,
        "topic": topic,
        "board_size": board,
        "round_count": round_count,
        "max_rounds": int(preset["max_rounds"]),
        "roles": roles,
        "problem_frame": str(problem_payload.get("problem_frame", "")).strip(),
        "constraints": [str(item).strip() for item in problem_payload.get("constraints", []) if str(item).strip()],
        "context_refs": [str(item).strip() for item in problem_payload.get("context_refs", []) if str(item).strip()],
        "options_seed": [str(item).strip() for item in problem_payload.get("options_seed", []) if str(item).strip()],
        "output_contract": {
            "required_fields": [
                "problem_frame",
                "options",
                "conflict_points",
                "decision",
                "why_not_others",
                "next_execution_step",
                "confidence",
            ]
        },
        "budget_summary": {
            "board_size": board,
            "round_count": round_count,
            "role_count": len(roles),
            "discussion_intensity": "large" if board == "large" else "bounded",
        },
    }
    return write_json(manifest_path, payload)


def synthesize_board(manifest_path: Path, role_notes_path: Path, output_path: Path | None) -> Path:
    manifest = load_json(manifest_path, {})
    role_notes = load_json(role_notes_path, [])
    if not isinstance(manifest, dict):
        raise ValueError("board manifest must be a JSON object")
    if not isinstance(role_notes, list):
        raise ValueError("role notes must be a JSON list")
    options: list[str] = []
    conflict_points: list[str] = []
    decisions: list[str] = []
    reasons_against: list[str] = []
    next_steps: list[str] = []
    for item in role_notes:
        if not isinstance(item, dict):
            continue
        for key, target in (
            ("options", options),
            ("conflict_points", conflict_points),
            ("why_not_others", reasons_against),
        ):
            values = item.get(key, [])
            if isinstance(values, list):
                target.extend(str(v).strip() for v in values if str(v).strip())
        decision = str(item.get("decision", "")).strip()
        if decision:
            decisions.append(decision)
        next_step = str(item.get("next_execution_step", "")).strip()
        if next_step:
            next_steps.append(next_step)
    chosen_decision = decisions[0] if decisions else "no_decision"
    artifact = {
        "generated_at": now_utc(),
        "task_id": str(manifest.get("task_id", "")).strip(),
        "topic": str(manifest.get("topic", "")).strip(),
        "board_size": str(manifest.get("board_size", "")).strip(),
        "round_count": int(manifest.get("round_count", 1) or 1),
        "roles": [item.get("role") for item in manifest.get("roles", []) if isinstance(item, dict)],
        "problem_frame": str(manifest.get("problem_frame", "")).strip(),
        "constraints": manifest.get("constraints", []),
        "options": sorted(dict.fromkeys(options)),
        "conflict_points": sorted(dict.fromkeys(conflict_points)),
        "decision": chosen_decision,
        "why_not_others": sorted(dict.fromkeys(reasons_against)),
        "next_execution_step": next_steps[0] if next_steps else "",
        "confidence": "medium" if len(decisions) <= 1 else "high",
        "budget_summary": manifest.get("budget_summary", {}),
    }
    out = output_path or manifest_path.with_name(manifest_path.name.replace(".board.json", ".decision.json"))
    return write_json(out, artifact)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    render = sub.add_parser("render")
    render.add_argument("task_id")
    render.add_argument("topic")
    render.add_argument("--board", choices=sorted(BOARD_PRESETS), default="small")
    render.add_argument("--rounds", type=int)
    render.add_argument("--problem-file")
    render.add_argument("--output-dir")

    synthesize = sub.add_parser("synthesize")
    synthesize.add_argument("board_manifest")
    synthesize.add_argument("role_notes")
    synthesize.add_argument("--output")

    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.command == "render":
        problem_payload = load_json(Path(args.problem_file), {}) if args.problem_file else {}
        output_dir = Path(args.output_dir).expanduser() if args.output_dir else None
        path = render_board(
            args.task_id,
            args.topic,
            board=args.board,
            rounds=args.rounds,
            problem_payload=problem_payload if isinstance(problem_payload, dict) else {},
            output_dir=output_dir,
        )
        print(str(path))
        return 0
    if args.command == "synthesize":
        path = synthesize_board(
            Path(args.board_manifest).expanduser(),
            Path(args.role_notes).expanduser(),
            Path(args.output).expanduser() if args.output else None,
        )
        print(str(path))
        return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
