#!/usr/bin/env python3
"""Emit a compact machine-readable VIDA boot packet."""

from __future__ import annotations

import json
import importlib.util
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent.parent
VIDA_CONFIG_PATH = SCRIPT_DIR / "vida-config.py"
VIDA_CONFIG_SPEC = importlib.util.spec_from_file_location("vida_config_boot_packet", VIDA_CONFIG_PATH)
if VIDA_CONFIG_SPEC is None or VIDA_CONFIG_SPEC.loader is None:
    raise RuntimeError(f"Unable to load VIDA config helper: {VIDA_CONFIG_PATH}")
vida_config = importlib.util.module_from_spec(VIDA_CONFIG_SPEC)
VIDA_CONFIG_SPEC.loader.exec_module(vida_config)


COMMON_READS = [
    "AGENTS.md",
    "_vida/docs/thinking-protocol.md#section-algorithm-selector",
    "_vida/docs/thinking-protocol.md#section-stc",
    "_vida/docs/thinking-protocol.md#section-pr-cot",
    "_vida/docs/thinking-protocol.md#section-mar",
    "_vida/docs/thinking-protocol.md#section-5-solutions",
    "_vida/docs/thinking-protocol.md#section-meta-analysis",
    "_vida/docs/thinking-protocol.md#section-bug-reasoning",
    "_vida/docs/thinking-protocol.md#section-web-search",
    "_vida/docs/thinking-protocol.md#section-reasoning-modules",
    "_vida/docs/web-validation-protocol.md",
    "_vida/docs/beads-protocol.md",
    "_vida/docs/project-overlay-protocol.md",
]

STANDARD_READS = [
    "_vida/docs/todo-protocol.md",
    "_vida/docs/implement-execution-protocol.md",
    "_vida/docs/use-case-packs.md",
]

FULL_READS = [
    "_vida/docs/orchestration-protocol.md",
    "_vida/docs/pipelines.md",
]


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def profile_reads(profile: str, non_dev: bool, agent_system_active: bool) -> list[str]:
    reads = list(COMMON_READS)
    if profile in {"standard", "full"}:
        reads.extend(STANDARD_READS)
    if profile == "full":
        reads.extend(FULL_READS)
    if non_dev:
        reads.append("_vida/docs/spec-contract-protocol.md")
    if Path(ROOT_DIR / "vida.config.yaml").exists():
        reads.append("vida.config.yaml")
    if agent_system_active:
        reads.append("_vida/docs/subagent-system-protocol.md")
    return reads


def packet_for(profile: str, non_dev: bool) -> dict[str, Any]:
    config = vida_config.load_validated_config() if Path(ROOT_DIR / "vida.config.yaml").exists() else {}
    language_policy = vida_config.dotted_get(config, "language_policy", {}) or {}
    protocol_activation = vida_config.dotted_get(config, "protocol_activation", {}) or {}
    agent_system_active = bool(protocol_activation.get("agent_system", False))
    return {
        "generated_at": now_utc(),
        "profile": profile,
        "non_dev": non_dev,
        "language_policy": language_policy,
        "protocol_activation": protocol_activation,
        "read_contract": profile_reads(profile, non_dev, agent_system_active),
        "invariants": [
            "read AGENTS.md first after compression",
            "apply thinking-protocol algorithms",
            "task state lives in br",
            "execute only through TODO blocks",
            "external-first fanout for eligible read-only work",
        ],
    }


def read_contract_for(profile: str, non_dev: bool) -> list[str]:
    config = vida_config.load_validated_config() if Path(ROOT_DIR / "vida.config.yaml").exists() else {}
    protocol_activation = vida_config.dotted_get(config, "protocol_activation", {}) or {}
    agent_system_active = bool(protocol_activation.get("agent_system", False))
    return profile_reads(profile, non_dev, agent_system_active)


def usage() -> int:
    print(
        "Usage: python3 _vida/scripts/boot-packet.py <lean|standard|full|read-contract|summary> [args]",
        file=sys.stderr,
    )
    return 1


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        return usage()
    command = argv[1].strip().lower()
    if command == "read-contract":
        if len(argv) < 3:
            return usage()
        profile = argv[2].strip().lower()
        if profile not in {"lean", "standard", "full"}:
            return usage()
        non_dev = "--non-dev" in argv[3:]
        for entry in read_contract_for(profile, non_dev):
            print(entry)
        return 0
    if command == "summary":
        if len(argv) < 3:
            return usage()
        subject = argv[2].strip()
        latest = ROOT_DIR / ".vida" / "logs" / "boot-receipts" / f"{subject}.latest.boot-packet.json"
        if not latest.exists():
            print(f"[boot-packet] Missing packet: {latest}", file=sys.stderr)
            return 1
        payload = json.loads(latest.read_text())
        print(
            json.dumps(
                {
                    "subject": subject,
                    "profile": payload.get("profile"),
                    "non_dev": payload.get("non_dev"),
                    "read_contract_count": len(payload.get("read_contract") or []),
                    "invariants_count": len(payload.get("invariants") or []),
                    "protocol_activation": payload.get("protocol_activation", {}),
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 0
    profile = command
    if profile not in {"lean", "standard", "full"}:
        return usage()
    non_dev = "--non-dev" in argv[2:]
    print(json.dumps(packet_for(profile, non_dev), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
