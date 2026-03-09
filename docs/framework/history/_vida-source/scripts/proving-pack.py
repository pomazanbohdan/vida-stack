#!/usr/bin/env python3
"""Canonical reusable proving-pack templates for product and framework regression work."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone


PACKS = {
    "navigation_ownership": {
        "goal": "prove route intent, redirect ownership, and top-level navigation stability",
        "checks": [
            "entry route resolves to intended module",
            "active navigation state matches exact route intent",
            "redirect logic does not remount unrelated sections",
            "deep-link and back-stack behavior remain stable",
        ],
    },
    "account_switch": {
        "goal": "prove account/session switch keeps ownership, scope, and visible state coherent",
        "checks": [
            "account switch updates session-bound data",
            "navigation ownership remains correct after switch",
            "no stale first-account data remains visible",
            "post-switch recovery does not widen into unrelated UI regressions",
        ],
    },
    "locale_preservation": {
        "goal": "prove locale changes preserve state and do not trigger unwanted remount side effects",
        "checks": [
            "locale update changes visible copy",
            "current route/module remains stable across locale update",
            "stateful widgets keep expected ownership",
            "no duplicate navigation or reset side effects occur",
        ],
    },
    "drawer_interaction": {
        "goal": "prove drawer spacing, dismissal, and first-level module navigation behavior",
        "checks": [
            "drawer opens with correct spacing/layout",
            "dismiss gestures and taps close predictably",
            "first-level module navigation lands on exact intended route",
            "drawer interactions do not desynchronize bottom-nav or header state",
        ],
    },
    "framework_self": {
        "goal": "prove VIDA framework runtime law, routing, and diagnostics remain fail-closed",
        "checks": [
            "execution auth gate blocks missing receipts/contracts",
            "cheap lanes fail closed on low-fitness outputs",
            "mutation path stays within queue-backed single writer",
            "silent diagnosis mode is visible in boot/runtime status",
        ],
    },
}


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("pack", choices=sorted(PACKS.keys()))
    parser.add_argument("--task-id", default="")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    payload = {
        "generated_at": now_utc(),
        "pack": args.pack,
        "task_id": args.task_id,
        **PACKS[args.pack],
    }
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
